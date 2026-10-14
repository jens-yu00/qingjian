"""用公开同音词样例测 MLX 首选决策；不读写个人学习数据，不接管输入法。"""
import argparse
import copy
import importlib.metadata
import json
import math
import resource
import statistics
import time
from pathlib import Path

import mlx.core as mx
from mlx_lm import load
from mlx_lm.models.cache import make_prompt_cache

SYSTEM = "你是中文输入法。根据上文和拼音，从候选中选出最适合接在上文后的词。候选顺序不代表偏好。只输出选中项的大写字母，不解释。"


def stats(rows):
    values = sorted(r["ms"] for r in rows)
    return {"n": len(rows), "p50_ms": statistics.median(values),
            "p95_ms": values[math.ceil(len(values) * .95) - 1], "max_ms": values[-1],
            "correct": sum(r["correct"] for r in rows),
            "habit_2x_correct": sum(r["habit_2x_correct"] for r in rows)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--rounds", type=int, default=3)
    parser.add_argument("--cases", default=str(Path(__file__).with_name("cases.json")))
    args = parser.parse_args()
    cases = json.loads(Path(args.cases).read_text())
    mx.set_cache_limit(256 * 1024 * 1024)
    started = time.perf_counter()
    model, tokenizer = load(args.model)
    mx.synchronize()
    load_ms = (time.perf_counter() - started) * 1000
    label_ids = [tokenizer.encode(c, add_special_tokens=False) for c in "ABCDEFGHIJKL"]
    assert all(len(ids) == 1 for ids in label_ids)
    label_ids = [ids[0] for ids in label_ids]

    def make_tokens(case, order):
        user = "上文：" + case["before"] + "\n拼音：" + case["pinyin"] + "\n候选：\n"
        user += "\n".join(f"{chr(65 + i)}. {word}" for i, word in enumerate(order))
        return tokenizer.apply_chat_template(
            [{"role": "system", "content": SYSTEM}, {"role": "user", "content": user}],
            tokenize=True, add_generation_prompt=True, enable_thinking=False)

    cached_tokens = {}

    def prepare(tokens):
        cache = make_prompt_cache(model)
        started = time.perf_counter()
        for offset in range(0, len(tokens), 512):
            model(mx.array(tokens[offset:offset + 512])[None], cache=cache)
            mx.eval([c.state for c in cache])
        mx.synchronize()
        cached_tokens[id(cache)] = list(tokens)
        return cache, (time.perf_counter() - started) * 1000

    def decide(case, order, prefix=None, prefix_len=0):
        started = time.perf_counter()
        tokens = make_tokens(case, order)
        if prefix is not None:
            assert tokens[:prefix_len] == cached_tokens[id(prefix)]
        cache = copy.deepcopy(prefix) if prefix is not None else make_prompt_cache(model)
        remaining = tokens[prefix_len:]
        output = model(mx.array(remaining)[None], cache=cache)[:, -1, :].astype(mx.float32)
        ids = mx.array(label_ids[:len(order)])
        logits = output[0, ids]
        probabilities = mx.softmax(logits)
        raw_top = mx.argmax(output[0])
        mass = mx.exp(mx.logsumexp(logits) - mx.logsumexp(output[0]))
        mx.eval(probabilities, raw_top, mass)
        mx.synchronize()
        elapsed = (time.perf_counter() - started) * 1000
        probs = probabilities.tolist()
        ranked = sorted(range(len(probs)), key=lambda i: probs[i], reverse=True)
        chosen = order[ranked[0]]
        habit_winner = max(range(len(probs)), key=lambda i: probs[i] * (2 if order[i] == case.get("baseline_first") else 1))
        return {"id": case["id"], "ms": elapsed, "tokens": len(tokens),
                "uncached_tokens": len(remaining), "chosen": chosen,
                "correct": chosen == case["expected"],
                "habit_2x_chosen": order[habit_winner],
                "habit_2x_correct": order[habit_winner] == case["expected"],
                "probabilities": dict(zip(order, probs)), "label_mass": mass.item(),
                "unconstrained_token": tokenizer.decode([raw_top.item()]),
                "peak_mlx_gb": mx.get_peak_memory() / 1e9}

    cold = decide(cases[0], cases[0]["candidates"])
    print(json.dumps({"load_ms": load_ms, "first_query": cold}, ensure_ascii=False), flush=True)
    all_tokens = [make_tokens(c, c["candidates"]) for c in cases]
    common_len = 0
    for columns in zip(*all_tokens):
        if len(set(columns)) != 1:
            break
        common_len += 1
    prefix, prefix_ms = prepare(all_tokens[0][:common_len])
    rows = []
    for mode in ["uncached", "system_prefix", "context_prefix"]:
        for case in cases:
            local_prefix, local_len, context_setup_ms = prefix, common_len, 0
            if mode == "context_prefix":
                # 仅预先缓存固定上文；候选、拼音和结果必须每次重新计算。
                empty = {**case, "pinyin": "", "candidates": []}
                empty_tokens = make_tokens(empty, [])
                local_len = 0
                for left, right in zip(make_tokens(case, case["candidates"]), empty_tokens):
                    if left != right:
                        break
                    local_len += 1
                local_prefix, context_setup_ms = prepare(make_tokens(case, case["candidates"])[:local_len])
            for repeat in range(args.rounds):
                order = list(case["candidates"])
                shift = repeat % len(order)
                order = order[shift:] + order[:shift]
                result = decide(case, order, None if mode == "uncached" else local_prefix,
                                0 if mode == "uncached" else local_len)
                result.update(mode=mode, repeat=repeat, context_setup_ms=context_setup_ms)
                rows.append(result)
        print(mode, json.dumps(stats([r for r in rows if r["mode"] == mode])), flush=True)
    # 较长的新上文仍只复用系统前缀，避免把预计算成本隐藏进结果。
    long_text = ("我们正在整理本周的工作记录，会议中讨论了项目进度、人员安排和下一阶段计划。" * 8)[-256:]
    for case in cases[:4]:
        expanded = {**case, "before": long_text + case["before"]}
        for repeat in range(args.rounds):
            result = decide(expanded, case["candidates"], prefix, common_len)
            result.update(mode="new_long_context", repeat=repeat)
            rows.append(result)
    report = {"model": args.model, "cases": args.cases, "mlx_lm": importlib.metadata.version("mlx-lm"),
              "mlx": importlib.metadata.version("mlx"), "load_ms": load_ms, "cold": cold,
              "prefix_tokens": common_len, "prefix_setup_ms": prefix_ms,
              "summary": {mode: stats([r for r in rows if r["mode"] == mode])
                          for mode in sorted({r["mode"] for r in rows})},
              "peak_mlx_gb": mx.get_peak_memory() / 1e9,
              "peak_rss_gb": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1e9,
              "rows": rows}
    Path(args.out).write_text(json.dumps(report, ensure_ascii=False, indent=2))
    print(json.dumps({k: v for k, v in report.items() if k != "rows"}, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
