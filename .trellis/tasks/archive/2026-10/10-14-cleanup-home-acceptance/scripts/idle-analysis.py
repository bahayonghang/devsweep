"""Analyze the frozen RT-AC7 five-window protocol without changing raw samples."""
import json
import math
from pathlib import Path
import statistics
import sys


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def write(path, data):
    with path.open("x", encoding="utf-8") as stream:
        json.dump(data, stream, indent=2)
        stream.write("\n")


def analyze(output):
    protocol = read(output / "protocol.json")
    raw = output / "measurement"
    launch, lifecycle = read(raw / "launch.json"), read(raw / "summary.json")
    assert launch["debug_port"] is None and lifecycle["debug_port"] is None
    assert lifecycle["sha256"] == protocol["exe_sha256"]
    assert lifecycle["lifecycle_pass"] and lifecycle["exit_code"] == 0
    assert not lifecycle["forced_teardown"] and not lifecycle["owned_after_exit"]
    assert lifecycle["close_requested"]["wall_ms"] >= 548000
    samples = [json.loads(line) for line in (raw / "samples.jsonl").read_text(encoding="utf-8").splitlines()]
    root = (launch["pid"], int(launch["created"]))
    repetitions, tree_windows = [], []
    for repetition in range(5):
        start = 128000 + repetition * 90000
        retained = [row for row in samples if start <= row["wall_ms"] < start + 60000]
        assert len(retained) >= 2, "unavailable retained samples are never zero CPU"
        roots = []
        identities = []
        for sample in retained:
            processes = sample["processes"]
            host = next(row for row in processes if (row["pid"], row.get("created")) == root)
            assert host["present"] and not host["unavailable"]
            assert all(row["present"] and not row["unavailable"] for row in processes)
            roots.append(host)
            identities.append({(row["pid"], row["created"]) for row in processes})
        intervals = [retained[i]["wall_ms"] - retained[i - 1]["wall_ms"] for i in range(1, len(retained))]
        deltas = [roots[i]["cpu_ms"] - roots[i - 1]["cpu_ms"] for i in range(1, len(roots))]
        assert all(value > 0 for value in intervals) and all(value >= 0 for value in deltas)
        cpu = [value / interval * 100 for value, interval in zip(deltas, intervals)]
        repetitions.append({"repetition": repetition + 1, "retained_start_ms": start, "retained_end_ms": start + 60000,
                            "sample_count": len(retained), "cpu_median": statistics.median(cpu),
                            "first_sample_wall_ms": retained[0]["wall_ms"], "last_sample_wall_ms": retained[-1]["wall_ms"],
                            "observed_span_ms": retained[-1]["wall_ms"] - retained[0]["wall_ms"],
                            "cpu_p95": sorted(cpu)[math.ceil(len(cpu) * .95) - 1],
                            "sample_gap_max_ms": max(intervals),
                            "peak_private_bytes": max(row["private_bytes"] for row in roots),
                            "peak_threads": max(row["threads"] for row in roots),
                            "tree_peak_private_bytes": max(sum(row["private_bytes"] for row in s["processes"]) for s in retained),
                            "tree_peak_threads": max(sum(row["threads"] for row in s["processes"]) for s in retained),
                            "tree_peak_processes": max(len(s["processes"]) for s in retained)})
        stable = all(identity == identities[0] for identity in identities)
        duration = retained[-1]["wall_ms"] - retained[0]["wall_ms"]
        tree_cpu = None
        if stable:
            tree_delta = sum(row["cpu_ms"] for row in retained[-1]["processes"]) - sum(row["cpu_ms"] for row in retained[0]["processes"])
            assert tree_delta >= 0
            tree_cpu = tree_delta / duration * 100
        tree_windows.append({"repetition": repetition + 1, "duration_ms": duration,
                             "host_interval_cpu_percent": (roots[-1]["cpu_ms"] - roots[0]["cpu_ms"]) / duration * 100,
                             "tree_interval_cpu_percent": tree_cpu, "tree_identity_set_stable": stable,
                             "tree_cpu_status": "available" if stable else "unavailable: identity set changed"})
    median = statistics.median(row["cpu_median"] for row in repetitions)
    private = max(row["peak_private_bytes"] for row in repetitions)
    threads = max(row["peak_threads"] for row in repetitions)
    gates = {"median_cpu_le_1": median <= 1, "peak_private_le_192mib": private <= 192 * 1024 * 1024, "peak_threads_le_40": threads <= 40}
    result = {"status": "PASS" if all(gates.values()) else "FAIL", "repetitions": repetitions,
              "cpu_median_of_run_medians": median, "peak_private_bytes": private, "peak_threads": threads,
              "gates": gates, "lifecycle_pass": True, "root_pid": root[0], "root_created": str(root[1]), "no_debugger": True,
              "protocol_coverage_status": "REVIEW_REQUIRED: sample count, first/last coverage and maximum gap must be checked against 200 ms protocol"}
    write(output / "idle-summary.json", result)
    write(output / "tree-resources.json", {"cpu_definition": "100%=one logical core; integrated retained-window delta, separate from inherited host gates",
                                          "windows": tree_windows, "exit_code": lifecycle["exit_code"],
                                          "forced_teardown": lifecycle["forced_teardown"], "owned_after_exit": lifecycle["owned_after_exit"],
                                          "helper_settle_ms": lifecycle["post_exit_settle"]["elapsed_ms"]})
    print(json.dumps({"status": result["status"], "gates": gates, "output": str(output)}))
    assert all(gates.values()), "formal inherited host resource gate failed"


if __name__ == "__main__":
    analyze(Path(sys.argv[1]))
