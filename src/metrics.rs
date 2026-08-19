use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceSample {
    pub cpu_usage_usec: u64,
    pub cpu_user_usec: u64,
    pub cpu_system_usec: u64,
    pub cpu_throttled_usec: u64,
    pub memory_current_bytes: u64,
    pub memory_peak_bytes: Option<u64>,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
    pub process_count: usize,
    pub thread_count: usize,
    pub file_descriptor_count: usize,
    pub cpu_pressure_some_avg10: Option<f64>,
    pub memory_pressure_some_avg10: Option<f64>,
    pub io_pressure_some_avg10: Option<f64>,
}

impl ResourceSample {
    pub fn delta(&self, before: &Self) -> Self {
        Self {
            cpu_usage_usec: self.cpu_usage_usec.saturating_sub(before.cpu_usage_usec),
            cpu_user_usec: self.cpu_user_usec.saturating_sub(before.cpu_user_usec),
            cpu_system_usec: self.cpu_system_usec.saturating_sub(before.cpu_system_usec),
            cpu_throttled_usec: self
                .cpu_throttled_usec
                .saturating_sub(before.cpu_throttled_usec),
            memory_current_bytes: self.memory_current_bytes,
            memory_peak_bytes: self.memory_peak_bytes,
            io_read_bytes: self.io_read_bytes.saturating_sub(before.io_read_bytes),
            io_write_bytes: self.io_write_bytes.saturating_sub(before.io_write_bytes),
            process_count: self.process_count,
            thread_count: self.thread_count,
            file_descriptor_count: self.file_descriptor_count,
            cpu_pressure_some_avg10: self.cpu_pressure_some_avg10,
            memory_pressure_some_avg10: self.memory_pressure_some_avg10,
            io_pressure_some_avg10: self.io_pressure_some_avg10,
        }
    }
}

pub fn sample(cgroup: &Path) -> Result<ResourceSample> {
    let cpu = parse_key_values(&read(cgroup.join("cpu.stat"))?);
    let io = parse_io(&read(cgroup.join("io.stat")).unwrap_or_default());
    let pids = read(cgroup.join("cgroup.procs"))?
        .lines()
        .filter_map(|line| line.trim().parse::<u32>().ok())
        .collect::<Vec<_>>();
    let (thread_count, file_descriptor_count) = process_details(&pids);
    Ok(ResourceSample {
        cpu_usage_usec: get(&cpu, "usage_usec"),
        cpu_user_usec: get(&cpu, "user_usec"),
        cpu_system_usec: get(&cpu, "system_usec"),
        cpu_throttled_usec: get(&cpu, "throttled_usec"),
        memory_current_bytes: read_u64(cgroup.join("memory.current"))?,
        memory_peak_bytes: read_u64(cgroup.join("memory.peak")).ok(),
        io_read_bytes: io.0,
        io_write_bytes: io.1,
        process_count: pids.len(),
        thread_count,
        file_descriptor_count,
        cpu_pressure_some_avg10: pressure_avg10(
            &read(cgroup.join("cpu.pressure")).unwrap_or_default(),
        ),
        memory_pressure_some_avg10: pressure_avg10(
            &read(cgroup.join("memory.pressure")).unwrap_or_default(),
        ),
        io_pressure_some_avg10: pressure_avg10(
            &read(cgroup.join("io.pressure")).unwrap_or_default(),
        ),
    })
}

fn process_details(pids: &[u32]) -> (usize, usize) {
    pids.iter().fold((0, 0), |(threads, fds), pid| {
        let task = std::fs::read_dir(format!("/proc/{pid}/task"))
            .map(|v| v.count())
            .unwrap_or(0);
        let fd = std::fs::read_dir(format!("/proc/{pid}/fd"))
            .map(|v| v.count())
            .unwrap_or(0);
        (threads + task, fds + fd)
    })
}

fn read(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
}

fn read_u64(path: impl AsRef<Path>) -> Result<u64> {
    let raw = read(path)?;
    raw.trim()
        .parse()
        .with_context(|| format!("parsing integer {raw:?}"))
}

fn parse_key_values(input: &str) -> BTreeMap<String, u64> {
    input
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some((fields.next()?.to_owned(), fields.next()?.parse().ok()?))
        })
        .collect()
}

fn get(values: &BTreeMap<String, u64>, key: &str) -> u64 {
    values.get(key).copied().unwrap_or_default()
}

fn parse_io(input: &str) -> (u64, u64) {
    let mut read_bytes = 0;
    let mut write_bytes = 0;
    for field in input.split_whitespace() {
        if let Some(value) = field
            .strip_prefix("rbytes=")
            .and_then(|v| v.parse::<u64>().ok())
        {
            read_bytes += value;
        }
        if let Some(value) = field
            .strip_prefix("wbytes=")
            .and_then(|v| v.parse::<u64>().ok())
        {
            write_bytes += value;
        }
    }
    (read_bytes, write_bytes)
}

fn pressure_avg10(input: &str) -> Option<f64> {
    let some = input.lines().find(|line| line.starts_with("some "))?;
    some.split_whitespace().find_map(|field| {
        field
            .strip_prefix("avg10=")
            .and_then(|value| value.parse().ok())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cpu_stat() {
        let values = parse_key_values("usage_usec 100\nuser_usec 60\nsystem_usec 40\n");
        assert_eq!(get(&values, "usage_usec"), 100);
    }

    #[test]
    fn sums_io_devices() {
        assert_eq!(
            parse_io("8:0 rbytes=4 wbytes=7\n8:1 rbytes=6 wbytes=3"),
            (10, 10)
        );
    }

    #[test]
    fn parses_pressure() {
        assert_eq!(
            pressure_avg10("some avg10=1.25 avg60=0.5 total=1"),
            Some(1.25)
        );
    }
}
