"""用手算的纠正、误改和合理变体核对统计口径。"""
import unittest
from analyze import metrics, predictions, wilson95


class MetricsTest(unittest.TestCase):
    def test_corrections_damage_and_acceptable_variants_are_distinct(self):
        cases = [
            dict(id='correct', baseline_first='上海', acceptable=['伤害']),
            dict(id='damage', baseline_first='食盐', acceptable=['食盐']),
            dict(id='variant', baseline_first='制定', acceptable=['制定', '制订']),
            dict(id='single', baseline_first='第一个', acceptable=['第一个']),
        ]
        result = metrics(cases, {'correct':'伤害', 'damage':'誓言', 'variant':'制订'})
        self.assertEqual(result['correct'], 3)
        self.assertEqual(result['baseline_correct'], 3)
        self.assertEqual(result['corrected'], 1)
        self.assertEqual(result['damaged'], 1)
        self.assertEqual(result['changed'], 3)
        self.assertEqual(result['net'], 0)
        self.assertEqual(result['mis_promotion_rate'], 1/3)

    def test_zero_observed_errors_still_has_nonzero_upper_bound(self):
        low, high = wilson95(0, 100)
        self.assertAlmostEqual(low, 0)
        self.assertAlmostEqual(high, 0.0369935, places=6)
        self.assertIsNone(wilson95(0, 0))

    def test_margin_gate_preserves_baseline_below_threshold(self):
        rows = [dict(id='a', selected='食盐', baseline_first='实验', margin=.99),
                dict(id='b', selected='伤害', baseline_first='上海', margin=1)]
        self.assertEqual(predictions(rows, 1), {'a':'实验','b':'伤害'})


if __name__ == '__main__':
    unittest.main()
