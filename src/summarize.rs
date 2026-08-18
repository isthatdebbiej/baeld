use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::event::{Event, EventKind};

const HEADLINE_MIN_RUNS: u64 = 20;

#[derive(Debug, Default, Serialize)]
struct Aggregate {
    runs: u64,
    successes: u64,
    browser_cpu_usec: u64,
    browser_wait_cpu_usec: u64,
    browser_cpu_throttled_usec: u64,
    browser_memory_current_bytes: u64,
    browser_memory_peak_bytes: Option<u64>,
    browser_io_read_bytes: u64,
    browser_io_write_bytes: u64,
    browser_cpu_pressure_some_avg10: Option<f64>,
    browser_memory_pressure_some_avg10: Option<f64>,
    browser_io_pressure_some_avg10: Option<f64>,
    driver_cpu_usec: u64,
    governor_cpu_usec: u64,
    server_cpu_usec: u64,
    host_steal_ticks: u64,
    latency_ms: Vec<f64>,
    resume_latency_ms: Vec<f64>,
    reconnects: u64,
    sequence_gaps: u64,
    background_operations: u64,
}

#[derive(Debug, Serialize)]
struct SummaryRow {
    mechanism: String,
    workload: String,
    wait_ms: u64,
    concurrency: usize,
    runs: u64,
    successes: u64,
    success_rate: f64,
    cpu_seconds_per_success: Option<f64>,
    wait_cpu_seconds_per_success: Option<f64>,
    cpu_throttled_seconds_per_success: Option<f64>,
    mean_memory_current_bytes: u64,
    max_memory_peak_bytes: Option<u64>,
    io_read_bytes_per_success: Option<f64>,
    io_write_bytes_per_success: Option<f64>,
    max_cpu_pressure_some_avg10: Option<f64>,
    max_memory_pressure_some_avg10: Option<f64>,
    max_io_pressure_some_avg10: Option<f64>,
    net_cpu_seconds_per_success: Option<f64>,
    driver_cpu_seconds_per_success: Option<f64>,
    governor_cpu_seconds_per_success: Option<f64>,
    server_cpu_seconds_per_success: Option<f64>,
    host_steal_ticks: u64,
    median_latency_ms: Option<f64>,
    p95_latency_ms: Option<f64>,
    median_resume_ms: Option<f64>,
    reconnects: u64,
    sequence_gaps: u64,
    mean_background_operations: f64,
    compatibility: &'static str,
    evidence_level: &'static str,
    net_cpu_change_vs_baseline_pct: Option<f64>,
}

pub fn run(run: &Path, json: bool) -> Result<()> {
    let rows = summarize(run)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        print_table(rows);
    }
    Ok(())
}

fn summarize(run: &Path) -> Result<Vec<SummaryRow>> {
    let events_path = run.join("events.jsonl");
    if !events_path.exists() {
        bail!("{} does not contain events.jsonl", run.display());
    }
    let file = fs::File::open(&events_path)?;
    let mut groups: BTreeMap<(String, String, u64, usize), Aggregate> = BTreeMap::new();
    for (line_no, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        let event: Event = serde_json::from_str(&line)
            .with_context(|| format!("parsing {} line {}", events_path.display(), line_no + 1))?;
        if let EventKind::TaskFinished {
            workload,
            wait_ms,
            concurrency,
            success,
            latency_ms,
            browser_cpu_usec,
            browser_wait_cpu_usec,
            browser_cpu_throttled_usec,
            browser_memory_current_bytes,
            browser_memory_peak_bytes,
            browser_io_read_bytes,
            browser_io_write_bytes,
            browser_cpu_pressure_some_avg10,
            browser_memory_pressure_some_avg10,
            browser_io_pressure_some_avg10,
            driver_cpu_usec,
            governor_cpu_usec,
            server_cpu_usec,
            host_steal_ticks,
            resume_latency_ms,
            reconnects,
            sequence_gaps,
            background_operations,
            ..
        } = event.kind
        {
            let aggregate = groups
                .entry((event.mechanism.slug(), workload, wait_ms, concurrency))
                .or_default();
            aggregate.runs += 1;
            aggregate.successes += u64::from(success);
            aggregate.browser_cpu_usec += browser_cpu_usec;
            aggregate.browser_wait_cpu_usec += browser_wait_cpu_usec;
            aggregate.browser_cpu_throttled_usec += browser_cpu_throttled_usec;
            aggregate.browser_memory_current_bytes += browser_memory_current_bytes;
            aggregate.browser_memory_peak_bytes = max_option(
                aggregate.browser_memory_peak_bytes,
                browser_memory_peak_bytes,
            );
            aggregate.browser_io_read_bytes += browser_io_read_bytes;
            aggregate.browser_io_write_bytes += browser_io_write_bytes;
            aggregate.browser_cpu_pressure_some_avg10 = max_f64_option(
                aggregate.browser_cpu_pressure_some_avg10,
                browser_cpu_pressure_some_avg10,
            );
            aggregate.browser_memory_pressure_some_avg10 = max_f64_option(
                aggregate.browser_memory_pressure_some_avg10,
                browser_memory_pressure_some_avg10,
            );
            aggregate.browser_io_pressure_some_avg10 = max_f64_option(
                aggregate.browser_io_pressure_some_avg10,
                browser_io_pressure_some_avg10,
            );
            aggregate.driver_cpu_usec += driver_cpu_usec;
            aggregate.governor_cpu_usec += governor_cpu_usec;
            aggregate.server_cpu_usec += server_cpu_usec;
            aggregate.host_steal_ticks += host_steal_ticks;
            aggregate.latency_ms.push(latency_ms);
            aggregate.resume_latency_ms.push(resume_latency_ms);
            aggregate.reconnects += reconnects;
            aggregate.sequence_gaps += sequence_gaps;
            aggregate.background_operations += background_operations;
        }
    }

    let mut rows: Vec<_> = groups.into_iter().map(summary_row).collect();

    let baselines: BTreeMap<_, _> = rows
        .iter()
        .filter(|row| row.mechanism == "baseline")
        .filter_map(|row| {
            row.net_cpu_seconds_per_success
                .map(|cpu| ((row.workload.clone(), row.wait_ms, row.concurrency), cpu))
        })
        .collect();
    for row in &mut rows {
        row.net_cpu_change_vs_baseline_pct = row
            .net_cpu_seconds_per_success
            .zip(
                baselines
                    .get(&(row.workload.clone(), row.wait_ms, row.concurrency))
                    .copied(),
            )
            .and_then(|(current, baseline)| {
                (baseline > 0.0).then(|| (current - baseline) / baseline * 100.0)
            });
    }
    Ok(rows)
}

fn summary_row(
    ((mechanism, workload, wait_ms, concurrency), mut aggregate): (
        (String, String, u64, usize),
        Aggregate,
    ),
) -> SummaryRow {
    aggregate.latency_ms.sort_by(f64::total_cmp);
    aggregate.resume_latency_ms.sort_by(f64::total_cmp);
    SummaryRow {
        mechanism,
        workload,
        wait_ms,
        concurrency,
        runs: aggregate.runs,
        successes: aggregate.successes,
        success_rate: ratio(aggregate.successes, aggregate.runs),
        cpu_seconds_per_success: per_success(aggregate.browser_cpu_usec, aggregate.successes),
        wait_cpu_seconds_per_success: per_success(
            aggregate.browser_wait_cpu_usec,
            aggregate.successes,
        ),
        net_cpu_seconds_per_success: per_success(
            aggregate.browser_cpu_usec
                + aggregate.driver_cpu_usec
                + aggregate.governor_cpu_usec,
            aggregate.successes,
        ),
        cpu_throttled_seconds_per_success: per_success(
            aggregate.browser_cpu_throttled_usec,
            aggregate.successes,
        ),
        mean_memory_current_bytes: aggregate.browser_memory_current_bytes / aggregate.runs.max(1),
        max_memory_peak_bytes: aggregate.browser_memory_peak_bytes,
        io_read_bytes_per_success: per_success_raw(
            aggregate.browser_io_read_bytes,
            aggregate.successes,
        ),
        io_write_bytes_per_success: per_success_raw(
            aggregate.browser_io_write_bytes,
            aggregate.successes,
        ),
        max_cpu_pressure_some_avg10: aggregate.browser_cpu_pressure_some_avg10,
        max_memory_pressure_some_avg10: aggregate.browser_memory_pressure_some_avg10,
        max_io_pressure_some_avg10: aggregate.browser_io_pressure_some_avg10,
        driver_cpu_seconds_per_success: per_success(
            aggregate.driver_cpu_usec,
            aggregate.successes,
        ),
        governor_cpu_seconds_per_success: per_success(
            aggregate.governor_cpu_usec,
            aggregate.successes,
        ),
        server_cpu_seconds_per_success: per_success(
            aggregate.server_cpu_usec,
            aggregate.successes,
        ),
        host_steal_ticks: aggregate.host_steal_ticks,
        median_latency_ms: percentile(&aggregate.latency_ms, 0.5),
        p95_latency_ms: percentile(&aggregate.latency_ms, 0.95),
        median_resume_ms: percentile(&aggregate.resume_latency_ms, 0.5),
        reconnects: aggregate.reconnects,
        sequence_gaps: aggregate.sequence_gaps,
        mean_background_operations: ratio(aggregate.background_operations, aggregate.runs),
        compatibility: compatibility(&aggregate),
        evidence_level: if aggregate.runs < HEADLINE_MIN_RUNS {
            "development-only"
        } else {
            "headline-minimum-met"
        },
        net_cpu_change_vs_baseline_pct: None,
    }
}

fn print_table(rows: Vec<SummaryRow>) {
    println!(
        "{:<28} {:<14} {:>7} {:>5} {:>9} {:>12} {:>10} {:<19}",
        "mechanism", "workload", "wait", "conc", "success", "net cpu", "vs base", "compatibility"
    );
    for row in rows {
        println!(
            "{:<28} {:<14} {:>6}ms {:>5} {:>4}/{:<4} {:>12} {:>10} {:<19}",
            row.mechanism,
            row.workload,
            row.wait_ms,
            row.concurrency,
            row.successes,
            row.runs,
            row.net_cpu_seconds_per_success
                .map(|v| format!("{v:.4}s"))
                .unwrap_or_else(|| "n/a".into()),
            row.net_cpu_change_vs_baseline_pct
                .map(|v| format!("{v:+.1}%"))
                .unwrap_or_else(|| "n/a".into()),
            row.compatibility,
        );
    }
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn compatibility(aggregate: &Aggregate) -> &'static str {
    if aggregate.successes < aggregate.runs
        || aggregate.reconnects > 0
        || aggregate.sequence_gaps > 0
    {
        "failure-observed"
    } else {
        "no-failure-observed"
    }
}

fn per_success(usec: u64, successes: u64) -> Option<f64> {
    (successes > 0).then(|| usec as f64 / 1_000_000.0 / successes as f64)
}

fn per_success_raw(value: u64, successes: u64) -> Option<f64> {
    (successes > 0).then(|| value as f64 / successes as f64)
}

fn max_option(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (left, right) => left.or(right),
    }
}

fn max_f64_option(left: Option<f64>, right: Option<f64>) -> Option<f64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (left, right) => left.or(right),
    }
}

fn percentile(values: &[f64], p: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let index = ((values.len() - 1) as f64 * p).ceil() as usize;
    values.get(index).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_schema_one_fixture_and_marks_it_development_only() {
        let run = Path::new(env!("CARGO_MANIFEST_DIR")).join("analysis/fixtures");
        let rows = summarize(&run).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows
            .iter()
            .all(|row| row.evidence_level == "development-only"));
        assert!(rows
            .iter()
            .all(|row| row.compatibility == "no-failure-observed"));
        assert!(rows.iter().all(|row| row.mean_background_operations == 0.0));
    }

    #[test]
    fn any_application_failure_marks_the_cell_as_failure_observed() {
        let aggregate = Aggregate {
            runs: 3,
            successes: 2,
            ..Aggregate::default()
        };
        assert_eq!(compatibility(&aggregate), "failure-observed");

        let reconnect = Aggregate {
            runs: 3,
            successes: 3,
            reconnects: 1,
            ..Aggregate::default()
        };
        assert_eq!(compatibility(&reconnect), "failure-observed");
    }
}
