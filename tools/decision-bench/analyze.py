"""按独立语境汇总准确率、误改、净收益；只在开发集选择挑战门槛。"""
import argparse
import json
import math
import statistics
from pathlib import Path


def metrics(cases, predictions):
    correct = changed = corrected = damaged = baseline_correct = 0
    for case in cases:
        selected = predictions.get(case['id'], case['baseline_first'])
        base_ok = case['baseline_first'] in case['acceptable']
        ok = selected in case['acceptable']
        baseline_correct += base_ok
        correct += ok
        changed += selected != case['baseline_first']
        corrected += ok and not base_ok
        damaged += base_ok and not ok
    return {'n': len(cases), 'correct': correct, 'accuracy': correct / len(cases),
            'baseline_correct': baseline_correct, 'changed': changed,
            'corrected': corrected, 'damaged': damaged, 'net': corrected - damaged,
            'mis_promotion_rate': damaged / baseline_correct if baseline_correct else None}


def predictions(rows, threshold):
    return {r['id']: (r['selected'] if r['margin'] >= threshold else r['baseline_first'])
            for r in rows}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--cases', required=True)
    parser.add_argument('--report', required=True)
    parser.add_argument('--out', required=True)
    parser.add_argument('--label-review')
    args = parser.parse_args()
    cases = json.loads(Path(args.cases).read_text())
    if args.label_review:
        review = json.loads(Path(args.label_review).read_text())
        for change in review['changes']:
            case = next(c for c in cases if c['id'] == change['id'])
            case['acceptable'] += change['additional_acceptable']
    report = json.loads(Path(args.report).read_text())
    rows = report['rows']
    by_permutation = [[r for r in rows if r['permutation'] == i] for i in range(3)]
    dev = [c for c in cases if c['split'] == 'dev']
    # 固定网格只用开发集：三个位置版本均无新增错误，优先净纠正多者，再选较小门槛。
    grid = [0, .5, 1, 2, 4, 8, 16]
    trials = []
    for threshold in grid:
        results = [metrics(dev, predictions(rs, threshold)) for rs in by_permutation]
        trials.append({'threshold': threshold, 'permutations': results})
    eligible = [t for t in trials if all(m['damaged'] == 0 for m in t['permutations'])
                and t['permutations'][0]['net'] > 0]
    choice = max(eligible, key=lambda t: (t['permutations'][0]['net'], -t['threshold']), default=None)
    threshold = choice['threshold'] if choice else math.inf
    summary = {'label_review': args.label_review, 'calibration': trials, 'chosen_threshold': threshold if choice else None,
               'threshold_disabled': not bool(choice), 'splits': {}}
    raw = {r['id']: r['selected'] for r in by_permutation[0]}
    policy = predictions(by_permutation[0], threshold)
    for split in ['dev', 'test', 'all']:
        subset = [c for c in cases if split == 'all' or c['split'] == split]
        ids = {c['id'] for c in subset}
        variants = [[r for r in rs if r['id'] in ids] for rs in by_permutation]
        inconsistent = [c['id'] for c in subset if len({r['selected'] for r in rows if r['id'] == c['id']}) > 1]
        summary['splits'][split] = {
            'baseline': metrics(subset, {}), 'raw': metrics(subset, raw),
            'challenger': metrics(subset, policy),
            'in_pool': sum(c['expected_in_pool'] for c in subset),
            'raw_in_pool': metrics([c for c in subset if c['expected_in_pool']], raw),
            'permutations_raw': [metrics(subset, {r['id']: r['selected'] for r in rs}) for rs in variants],
            'permutations_challenger': [metrics(subset, predictions(rs, threshold)) for rs in variants],
            'order_sensitive_ids': inconsistent,
        }
    durations = sorted(r['ms'] for r in rows if not r['cold_request'])
    summary['performance'] = {
        'load_ms': report['load_ms'], 'first_request_ms': rows[0]['ms'],
        'warm_p50_ms': statistics.median(durations),
        'warm_p95_ms': durations[math.ceil(len(durations) * .95) - 1],
        'warm_n': len(durations), 'mlx_peak_bytes': report['mlx_peak_bytes'],
        'rss_peak_bytes': report['rss_peak_bytes'],
    }
    summary['failures'] = [dict(id=c['id'], before=c['before'], pinyin=c['pinyin'],
                                acceptable=c['acceptable'], baseline=c['baseline_first'],
                                selected=raw.get(c['id'],c['baseline_first']),
                                in_pool=c['expected_in_pool']) for c in cases
                           if raw.get(c['id'],c['baseline_first']) not in c['acceptable']]
    Path(args.out).write_text(json.dumps(summary, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({k:v for k,v in summary.items() if k!='calibration'}, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
