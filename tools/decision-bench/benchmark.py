"""专用决策头的中文同音选词离线评测；输入只含公开语境与实际候选。"""
import argparse
import hashlib
import importlib
import json
import math
import platform
import random
import resource
import sys
import time
from pathlib import Path

REVISION = '623e99bfb301402ac98ae11341510eda133ec901'
INSTRUCTION = '根据已经输入的中文上文，选择最适合紧接在上文之后的候选词。只判断接续是否自然且语义正确，不解释词义，不优先选择列表靠前的词。所有选项均来自当前拼音的候选。'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--model', required=True)
    parser.add_argument('--cases', required=True)
    parser.add_argument('--out', required=True)
    args = parser.parse_args()
    cases = json.loads(Path(args.cases).read_text())
    assert len({c['id'] for c in cases}) == len(cases)
    assert not ({c['pinyin'] for c in cases if c['split'] == 'dev'} &
                {c['pinyin'] for c in cases if c['split'] == 'test'})
    for case in cases:
        assert case['split'] in ['dev', 'test']
        assert len(case['candidates']) == len(set(case['candidates'])) >= 1
        assert case['acceptable'] and case['expected'] in case['acceptable']
    passthrough = [c for c in cases if len(c['candidates']) == 1]
    cases = [c for c in cases if len(c['candidates']) > 1]
    sys.path.insert(0, str(Path(args.model).resolve()))
    started = time.perf_counter()
    runtime = importlib.import_module('model')
    mx = runtime.mx
    mx.set_cache_limit(256 * 1024 * 1024)
    model, config, tokenizer = runtime.load_bundle(args.model)
    load_ms = (time.perf_counter() - started) * 1000
    output = Path(args.out)
    output.parent.mkdir(parents=True, exist_ok=True)
    report = {
        'collection_revision': REVISION,
        'source_model': config['source_repo'],
        'source_revision': config['source_revision'],
        'bundle_manifest_sha256': sha(Path(args.model) / 'MLX_MANIFEST.json'),
        'cases_sha256': sha(args.cases),
        'runner_sha256': sha(__file__),
        'instructions': INSTRUCTION,
        'load_ms': load_ms,
        'environment': {'python': sys.version, 'platform': platform.platform(),
                        'mlx': importlib.metadata.version('mlx'),
                        'mlx_lm': importlib.metadata.version('mlx-lm')},
        'passthrough_cases': [c['id'] for c in passthrough],
        'rows': [],
    }
    order = list(range(len(cases)))
    random.Random(20261014).shuffle(order)
    # 顺序置换只检查同例稳定性，不能把它们当成独立质量样本。
    with output.with_suffix('.jsonl').open('w') as stream:
        for permutation in range(3):
            for index in order:
                case = cases[index]
                candidates = list(case['candidates'])
                if permutation == 1:
                    candidates.reverse()
                elif permutation == 2:
                    random.Random(case['id']).shuffle(candidates)
                payload = {
                    'task_type': 'choice',
                    'state': {'上文': case['before'], '当前拼音': case['pinyin']},
                    'instructions': INSTRUCTION,
                    'options': [{'key': str(i), 'description': word}
                                for i, word in enumerate(candidates)],
                }
                started = time.perf_counter()
                encoded = runtime.encode(payload, tokenizer, config['max_input_tokens'])
                logits = model(mx.array([encoded['ids']], mx.int32),
                               encoded['candidate_positions'], encoded['query_position'])
                mx.eval(logits)
                result = runtime.answer(logits[0].tolist(), payload)
                selected = candidates[int(result['answer']['choice'])]
                elapsed = (time.perf_counter() - started) * 1000
                values = dict(zip(candidates, result['logits']))
                ranked = sorted(values, key=values.get, reverse=True)
                row = {
                    'id': case['id'], 'split': case['split'],
                    'permutation': permutation, 'candidates': candidates,
                    'selected': selected, 'correct': selected in case['acceptable'],
                    'baseline_first': case['baseline_first'],
                    'baseline_correct': case['baseline_first'] in case['acceptable'],
                    'expected_in_pool': case['expected_in_pool'],
                    'logits': values,
                    'probabilities': dict(zip(candidates, result['probabilities'].values())),
                    'margin': values[ranked[0]] - values[ranked[1]],
                    'ms': elapsed, 'tokens': len(encoded['ids']),
                    'token_ids_sha256': encoded['token_ids_sha256'],
                    'cold_request': not report['rows'],
                }
                assert all(math.isfinite(v) for v in values.values())
                report['rows'].append(row)
                stream.write(json.dumps(row, ensure_ascii=False) + '\n')
                stream.flush()
                if len(report['rows']) % 12 == 0:
                    print(f"{len(report['rows'])}/{len(cases)*3}: {case['id']} -> {selected} ({elapsed:.0f} ms)", flush=True)
    report['mlx_peak_bytes'] = mx.get_peak_memory()
    report['mlx_active_bytes'] = mx.get_active_memory()
    report['rss_peak_bytes'] = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    print('saved', output, flush=True)


if __name__ == '__main__':
    import importlib.metadata
    main()
