#!/usr/bin/env python3
import argparse
import json
import random
from collections import defaultdict
from pathlib import Path


def mechanism_slug(value):
    kind = value["kind"]
    if kind == "cpu-quota":
        return f"cpu-quota-{value['quota_us']}-{value['period_us']}"
    if kind == "cgroup-freeze":
        return f"cgroup-freeze-{value['delay_ms']}ms"
    return kind


def load_tasks(path):
    tasks = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        event = json.loads(line)
        if event.get("event") == "task_finished":
            event["mechanism"] = mechanism_slug(event["mechanism"])
            event["source_line"] = line_number
            event.setdefault("concurrency", 1)
            event.setdefault("block_id", 0)
            tasks.append(event)
    return tasks


def bootstrap_mean_ci(differences, samples=10_000, seed=20260817):
    rng = random.Random(seed)
    means = sorted(
        sum(rng.choice(differences) for _ in differences) / len(differences)
        for _ in range(samples)
    )
    return [means[int(samples * 0.025)], means[int(samples * 0.975)]]


def net_cpu(task):
    return task["browser_cpu_usec"] + task["driver_cpu_usec"] + task["governor_cpu_usec"]


def block_value(tasks, getter):
    successful = [task for task in tasks if task["success"]]
    if not successful:
        return None
    # Failed attempts remain in the numerator, making this correctness-adjusted
    # CPU per successful task rather than silently discarding their cost.
    return sum(getter(task) for task in tasks) / len(successful)


def analyze(tasks):
    groups = defaultdict(list)
    for task in tasks:
        groups[(task["workload"], task["wait_ms"], task["concurrency"], task["mechanism"])].append(task)

    rows = []
    cells = sorted({(task["workload"], task["wait_ms"], task["concurrency"]) for task in tasks})
    for workload, wait_ms, concurrency in cells:
        baseline = groups[(workload, wait_ms, concurrency, "baseline")]
        if not baseline:
            continue
        mechanisms = sorted({
            task["mechanism"] for task in tasks
            if task["workload"] == workload and task["wait_ms"] == wait_ms
            and task["concurrency"] == concurrency and task["mechanism"] != "baseline"
        })
        for mechanism in mechanisms:
            compared = groups[(workload, wait_ms, concurrency, mechanism)]
            has_block_ids = all(task["block_id"] > 0 for task in baseline + compared)
            if has_block_ids:
                baseline_blocks = defaultdict(list)
                compared_blocks = defaultdict(list)
                for task in baseline:
                    baseline_blocks[task["block_id"]].append(task)
                for task in compared:
                    compared_blocks[task["block_id"]].append(task)
                common_blocks = sorted(baseline_blocks.keys() & compared_blocks.keys())
                pairs = [(baseline_blocks[block], compared_blocks[block]) for block in common_blocks]
                pairing = "randomized block_id"
            else:
                if concurrency != 1 or len(compared) != len(baseline):
                    continue
                pairs = [([left], [right]) for left, right in zip(baseline, compared)]
                pairing = "legacy mechanism occurrence order (schema < 4)"
            for metric, getter in (
                ("net_cpu_seconds", net_cpu),
                ("browser_cpu_seconds", lambda task: task["browser_cpu_usec"]),
                ("browser_wait_cpu_seconds", lambda task: task.get("browser_wait_cpu_usec", 0)),
            ):
                values = [(block_value(left, getter), block_value(right, getter)) for left, right in pairs]
                differences = [
                    (right - left) / 1_000_000
                    for left, right in values
                    if left is not None and right is not None
                ]
                if not differences:
                    continue
                rows.append({
                    "workload": workload,
                    "wait_ms": wait_ms,
                    "concurrency": concurrency,
                    "mechanism": mechanism,
                    "metric": metric,
                    "pairs": len(differences),
                    "mean_difference_seconds": sum(differences) / len(differences),
                    "bootstrap_mean_difference_ci95": bootstrap_mean_ci(differences),
                    "pairing": pairing,
                })
    return rows


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    tasks = load_tasks(args.run / "events.jsonl")
    rows = analyze(tasks)
    output = args.output or args.run / "paired.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(rows, indent=2), encoding="utf-8")
    print(f"Wrote {output}")


if __name__ == "__main__":
    main()
