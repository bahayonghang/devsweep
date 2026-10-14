"""Correlate native Status joins with the owned 200 ms resource sampler."""
import argparse
import json
from pathlib import Path
import statistics


def summarize(lifecycle: dict, samples: list[dict]) -> dict:
    pid = lifecycle['launch']['pid']
    created = int(lifecycle['launch']['created'])
    rows = []
    previous = None
    for sample in samples:
        process = next((row for row in sample['processes']
                        if row['pid'] == pid and row.get('created') == created), None)
        if process is None or 'sampled_unix_ns' not in sample:
            continue
        current = {'at_ms': int(sample['sampled_unix_ns']) / 1_000_000,
                   'cpu_ms': process['cpu_ms'], 'threads': process['threads']}
        if previous and current['at_ms'] > previous['at_ms']:
            current['cpu_percent'] = ((current['cpu_ms'] - previous['cpu_ms']) /
                                      (current['at_ms'] - previous['at_ms']) * 100)
            rows.append(current)
        previous = current
    baseline = [row['cpu_percent'] for row in rows
                if lifecycle['pre_live_start_ms'] <= row['at_ms'] <= lifecycle['pre_live_end_ms']]
    if len(baseline) < 5:
        return {'status': 'unavailable', 'reason': 'fewer than five same-instance pre-live samples'}
    baseline_median = statistics.median(baseline)
    cycles = []
    for cycle in lifecycle['cycles']:
        joined = cycle['live']['settled_ms']
        post = [row for row in rows if joined < row['at_ms'] <= cycle['post_end_ms']][:25]
        record = {'cycle': cycle['cycle'], 'post_sample_count': len(post),
                  'ack_ms': cycle['cancel']['settled_ms'] - cycle['cancel']['request_ms'],
                  'join_ms': joined - cycle['cancel']['request_ms'],
                  'post_samples': post}
        if len(post) != 25:
            record.update(status='unavailable', reason='fewer than 25 scheduled post-join samples')
        else:
            final = [row['cpu_percent'] for row in post[-5:]]
            record.update(final_five_cpu_percent=final,
                          thread_floor=min(row['threads'] for row in post),
                          status='pass' if max(final) <= baseline_median + 0.5 else 'fail')
        cycles.append(record)
    floors = [cycle['thread_floor'] for cycle in cycles if 'thread_floor' in cycle]
    floor_status = ('unavailable' if len(floors) != 5 else
                    'pass' if floors[-1] <= floors[0] + 1 else 'fail')
    status = ('unavailable' if len(cycles) != 5 or floor_status == 'unavailable' or
              any(cycle['status'] == 'unavailable' for cycle in cycles) else
              'pass' if floor_status == 'pass' and all(cycle['status'] == 'pass' for cycle in cycles)
              else 'fail')
    return {'status': status, 'root_pid': pid, 'created': str(created),
            'cpu_definition': '100% equals one logical core',
            'pre_live_sample_count': len(baseline), 'pre_live_cpu_percent': baseline,
            'pre_live_median_cpu_percent': baseline_median,
            'post_cpu_ceiling_percent': baseline_median + 0.5,
            'comparison': 'every one of final five samples, not an averaged spike',
            'cycles': cycles, 'thread_floors': floors, 'thread_floor_status': floor_status}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lifecycle', type=Path)
    parser.add_argument('samples', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    result = summarize(json.loads(args.lifecycle.read_text()),
                       [json.loads(line) for line in args.samples.read_text().splitlines()])
    with args.output.open('x', encoding='utf-8') as stream:
        json.dump(result, stream, indent=2)
        stream.write('\n')
    print(json.dumps({'status': result['status'], 'output': str(args.output)}))
    if result['status'] != 'pass':
        raise SystemExit(1)
