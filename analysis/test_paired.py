import unittest

from analysis.paired import analyze


def task(mechanism, concurrency, block_id, cpu, success=True):
    return {
        "workload": "normal-spa",
        "wait_ms": 5000,
        "concurrency": concurrency,
        "block_id": block_id,
        "mechanism": mechanism,
        "success": success,
        "browser_cpu_usec": cpu,
        "browser_wait_cpu_usec": cpu // 2,
        "driver_cpu_usec": 0,
        "governor_cpu_usec": 0,
    }


class PairedAnalysisTests(unittest.TestCase):
    def test_keeps_concurrency_cells_separate_and_pairs_by_block(self):
        tasks = [
            task("baseline", 1, 1, 100),
            task("cpu-quota-25000-100000", 1, 1, 80),
            task("baseline", 5, 2, 500),
            task("cpu-quota-25000-100000", 5, 2, 400),
        ]
        rows = [row for row in analyze(tasks) if row["metric"] == "net_cpu_seconds"]
        self.assertEqual({row["concurrency"] for row in rows}, {1, 5})
        self.assertTrue(all(row["pairing"] == "randomized block_id" for row in rows))

    def test_failed_attempt_cost_stays_in_correctness_adjusted_numerator(self):
        tasks = [
            task("baseline", 2, 1, 100),
            task("baseline", 2, 1, 100),
            task("cpu-quota-25000-100000", 2, 1, 100),
            task("cpu-quota-25000-100000", 2, 1, 100, success=False),
        ]
        row = next(
            row for row in analyze(tasks)
            if row["metric"] == "net_cpu_seconds"
        )
        self.assertAlmostEqual(row["mean_difference_seconds"], 0.0001)


if __name__ == "__main__":
    unittest.main()
