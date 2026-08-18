#!/usr/bin/env python3
import argparse
import json
import math
import os
import random
from collections import defaultdict
from pathlib import Path

os.environ.setdefault("MPLCONFIGDIR", str(Path(".baeld/matplotlib").resolve()))
Path(os.environ["MPLCONFIGDIR"]).mkdir(parents=True, exist_ok=True)

import matplotlib.pyplot as plt
import numpy as np


def load_tasks(path):
    tasks = []
    with path.open(encoding="utf-8") as source:
        for line_no, line in enumerate(source, 1):
            event = json.loads(line)
            if event.get("event") != "task_finished":
                continue
            event["mechanism"] = mechanism_slug(event["mechanism"])
            event["source_line"] = line_no
            tasks.append(event)
    return tasks


def mechanism_slug(value):
    kind = value["kind"]
    if kind == "cpu-quota":
        return f"cpu-quota-{value['quota_us']}-{value['period_us']}"
    if kind == "cgroup-freeze":
        return f"cgroup-freeze-{value['delay_ms']}ms"
    return kind


def bootstrap_ci(values, statistic=np.median, samples=5000, seed=20260817):
    if not values:
        return None
    rng = random.Random(seed)
    estimates = []
    for _ in range(samples):
        draw = [rng.choice(values) for _ in values]
        estimates.append(float(statistic(draw)))
    estimates.sort()
    return estimates[int(samples * 0.025)], estimates[int(samples * 0.975)]


def summarize(tasks):
    groups = defaultdict(list)
    for task in tasks:
        key = (task["workload"], task["wait_ms"], task["mechanism"])
        groups[key].append(task)
    rows = []
    for key, values in sorted(groups.items()):
        successful = [v for v in values if v["success"]]
        cpu = [v["browser_cpu_usec"] / 1e6 for v in successful]
        wait_cpu = [v.get("browser_wait_cpu_usec", 0) / 1e6 for v in successful]
        total_cpu = sum(
            v["browser_cpu_usec"] + v["driver_cpu_usec"] + v["governor_cpu_usec"]
            for v in values
        ) / 1e6
        rows.append({
            "workload": key[0],
            "wait_ms": key[1],
            "mechanism": key[2],
            "runs": len(values),
            "successes": len(successful),
            "success_rate": len(successful) / len(values),
            "cpu_seconds_per_success": sum(cpu) / len(successful) if successful else None,
            "wait_cpu_seconds_per_success": sum(wait_cpu) / len(successful) if successful else None,
            "net_cpu_seconds_per_success": total_cpu / len(successful) if successful else None,
            "server_cpu_seconds": sum(v["server_cpu_usec"] for v in values) / 1e6,
            "host_steal_ticks": sum(v["host_steal_ticks"] for v in values),
            "median_latency_ms": float(np.median([v["latency_ms"] for v in values])),
            "p95_latency_ms": float(np.percentile([v["latency_ms"] for v in values], 95)),
            "cpu_median_ci95": bootstrap_ci(cpu),
            "wait_cpu_median_ci95": bootstrap_ci(wait_cpu),
            "reconnects": sum(v["reconnects"] for v in values),
            "sequence_gaps": sum(v["sequence_gaps"] for v in values),
        })
    return rows


def plot(rows, output):
    workloads = {row["workload"] for row in rows}
    workload = "agent-dashboard" if "agent-dashboard" in workloads else "normal-spa"
    representative = [r for r in rows if r["workload"] == workload and r["wait_ms"] == 5000]
    if not representative:
        return
    labels = [r["mechanism"] for r in representative]
    complete = [r["net_cpu_seconds_per_success"] or math.nan for r in representative]
    wait = [r["wait_cpu_seconds_per_success"] or math.nan for r in representative]
    fig, axes = plt.subplots(1, 2, figsize=(14, 5))
    axes[0].bar(labels, complete, color="#335c67")
    axes[0].set_ylabel("CPU seconds per successful task")
    axes[0].set_title("Complete-task net CPU (primary)")
    axes[1].bar(labels, wait, color="#9e2a2b")
    axes[1].set_ylabel("Browser CPU seconds during model wait")
    axes[1].set_title("Wait-window browser CPU (diagnostic)")
    for ax in axes:
        ax.tick_params(axis="x", rotation=25)
    fig.suptitle(f"Baeld {workload} (5 s model wait)")
    fig.tight_layout()
    fig.savefig(output, dpi=180)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    tasks = load_tasks(args.run / "events.jsonl")
    rows = summarize(tasks)
    output = args.output or args.run
    output.mkdir(parents=True, exist_ok=True)
    (output / "summary.json").write_text(json.dumps(rows, indent=2), encoding="utf-8")
    plot(rows, output / "representative-cpu.png")
    print(f"Wrote {output / 'summary.json'}")


if __name__ == "__main__":
    main()
