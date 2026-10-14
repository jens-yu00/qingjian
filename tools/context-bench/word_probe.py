"""对照直接输出候选词，检查字母标签是否引入了额外的选词错误。"""
import argparse
import copy
import json
import math
import statistics
import time
from pathlib import Path

import mlx.core as mx
from mlx_lm import load, stream_generate
from mlx_lm.models.cache import make_prompt_cache

SYSTEM = "你是中文输入法。根据上文和拼音，从候选词中选择最适合接在上文后的一个词。只输出该候选词，不解释，不输出编号。"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", required=True)
    parser.add_argument("--cases", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    cases = json.loads(Path(args.cases).read_text())
    mx.set_cache_limit(256 * 1024 * 1024)
    model, tokenizer = load(args.model)

    def tokens(case, order):
        user = f"上文：{case['before']}\n拼音：{case['pinyin']}\n候选词：" + "、".join(order)
        return tokenizer.apply_chat_template(
            [{"role": "system", "content": SYSTEM}, {"role": "user", "content": user}],
            tokenize=True, add_generation_prompt=True, enable_thinking=False)

    prompts = [tokens(c, c["candidates"]) for c in cases]
    common_len = next(i for i, parts in enumerate(zip(*prompts)) if len(set(parts)) != 1)
    prefix_tokens = prompts[0][:common_len]
    cache = make_prompt_cache(model)
    model(mx.array(prefix_tokens)[None], cache=cache)
    mx.eval([c.state for c in cache])
    mx.synchronize()

    def decide(case, order):
        started = time.perf_counter()
        prompt = tokens(case, order)
        assert prompt[:common_len] == prefix_tokens
        response = ""
        for part in stream_generate(model, tokenizer, prompt[common_len:], max_tokens=8,
                                    prompt_cache=copy.deepcopy(cache)):
            response += part.text
        mx.synchronize()
        elapsed = (time.perf_counter() - started) * 1000
        chosen = response.strip()
        return {"id": case["id"], "ms": elapsed, "chosen": chosen,
                "correct": chosen == case["expected"], "valid": chosen in order,
                "generated_tokens": part.generation_tokens}

    cold = decide(cases[0], cases[0]["candidates"])
    rows = []
    for case in cases:
        for repeat in range(3):
            order = case["candidates"][repeat:] + case["candidates"][:repeat]
            result = decide(case, order)
            rows.append({**result, "repeat": repeat})
        print(case["id"], [(r["chosen"], round(r["ms"])) for r in rows[-3:]], flush=True)
    values = sorted(r["ms"] for r in rows)
    report = {"model": args.model, "cases": args.cases, "cold": cold,
              "prefix_tokens": common_len, "n": len(rows),
              "correct": sum(r["correct"] for r in rows), "valid": sum(r["valid"] for r in rows),
              "p50_ms": statistics.median(values), "p95_ms": values[math.ceil(len(values) * .95) - 1],
              "peak_mlx_gb": mx.get_peak_memory() / 1e9, "rows": rows}
    Path(args.out).write_text(json.dumps(report, ensure_ascii=False, indent=2))
    print(json.dumps({k: v for k, v in report.items() if k != "rows"}, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
