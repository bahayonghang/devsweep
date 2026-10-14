"""Evidence must not pass missing post-stop samples or a hidden CPU spike."""
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location('status_summary', Path(__file__).parents[1] / 'summarize-native-status.py')
SUMMARY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SUMMARY)


def fixture():
    lifecycle = {'launch': {'pid': 10, 'created': '100'},
                 'pre_live_start_ms': 0, 'pre_live_end_ms': 5000, 'cycles': []}
    for index in range(5):
        joined = 10000 + index * 10000
        lifecycle['cycles'].append({'cycle': index, 'live': {'settled_ms': joined},
                                    'cancel': {'request_ms': joined - 100, 'settled_ms': joined - 90},
                                    'post_end_ms': joined + 5500})
    samples = [{'sampled_unix_ns': str(ms * 1000000),
                'processes': [{'pid': 10, 'created': 100, 'cpu_ms': 0, 'threads': 14}]}
               for ms in range(0, 56000, 200)]
    return lifecycle, samples


class NativeStatusSummaryTests(unittest.TestCase):
    def test_five_complete_cycles_pass_with_separate_ack_and_join(self):
        lifecycle, samples = fixture()
        result = SUMMARY.summarize(lifecycle, samples)
        self.assertEqual(result['status'], 'pass')
        self.assertEqual(result['thread_floors'], [14] * 5)
        self.assertEqual(result['cycles'][0]['ack_ms'], 10)
        self.assertEqual(result['cycles'][0]['join_ms'], 100)
        self.assertTrue(all(row['post_sample_count'] == 25 for row in result['cycles']))

    def test_short_or_wrong_identity_samples_are_unavailable(self):
        lifecycle, samples = fixture()
        self.assertEqual(SUMMARY.summarize(lifecycle, samples[:65])['status'], 'unavailable')
        for sample in samples:
            sample['processes'][0]['created'] = 101
        self.assertEqual(SUMMARY.summarize(lifecycle, samples)['status'], 'unavailable')

    def test_one_final_sample_cpu_spike_is_not_averaged_away(self):
        lifecycle, samples = fixture()
        for sample in samples:
            if int(sample['sampled_unix_ns']) >= 14800 * 1000000:
                sample['processes'][0]['cpu_ms'] = 2
        result = SUMMARY.summarize(lifecycle, samples)
        self.assertEqual(result['status'], 'fail')
        self.assertEqual(result['cycles'][0]['final_five_cpu_percent'][-2], 1)


if __name__ == '__main__':
    unittest.main()
