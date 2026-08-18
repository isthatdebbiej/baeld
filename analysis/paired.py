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


def analyze(tasks):
    groups = defaultdict(list)
    for task in tasks:
        groups[(task["workload"], task["wait_ms"], task["mechanism"])].append(task)

    rows = []
    cells = sorted({(task["workload"], task["wait_ms"]) for task in tasks})
    for workload, wait_ms in cells:
        baseline = groups[(workload, wait_ms, "baseline")]
        if not baseline:
            continue
        mechanisms = sorted({
            task["mechanism"] for task in tasks
            if task["workload"] == workload and task["wait_ms"] == wait_ms
            and task["mechanism"] != "baseline"
        })
        for mechanism in mechanisms:
            compared = groups[(workload, wait_ms, mechanism)]
            if len(compared) != len(baseline):
                continue
            pairs = list(zip(baseline, compared))
            for metric, getter in (
                ("net_cpu_seconds", net_cpu),
                ("browser_cpu_seconds", lambda task: task["browser_cpu_usec"]),
                ("browser_wait_cpu_seconds", lambda task: task.get("browser_wait_cpu_usec", 0)),
            ):
                differences = [(getter(right) - getter(left)) / 1_000_000 for left, right in pairs]
                rows.append({
                    "workload": workload,
                    "wait_ms": wait_ms,
                    "mechanism": mechanism,
                    "metric": metric,
                    "pairs": len(differences),
                    "mean_difference_seconds": sum(differences) / len(differences),
                    "bootstrap_mean_difference_ci95": bootstrap_mean_ci(differences),
                    "pairing": "mechanism occurrence order; valid only for concurrency-one paired blocks",
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
