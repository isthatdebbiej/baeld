use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::metrics::ResourceSample;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthState {
    Starting,
    Healthy,
    Degraded,
    Stuck,
    Terminating,
    Cleaned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSnapshot {
    pub state: HealthState,
    pub reasons: Vec<String>,
    pub resources: ResourceSample,
}

pub struct HealthMonitor {
    state: HealthState,
    consecutive_degraded: u8,
    consecutive_cdp_failures: u8,
    memory_limit: u64,
    process_limit: usize,
    phase_timeout: Duration,
    last_phase: Instant,
}

impl HealthMonitor {
    pub fn new(memory_limit: u64, process_limit: usize, phase_timeout: Duration) -> Self {
        Self {
            state: HealthState::Starting,
            consecutive_degraded: 0,
            consecutive_cdp_failures: 0,
            memory_limit,
            process_limit,
            phase_timeout,
            last_phase: Instant::now(),
        }
    }

    pub fn phase_seen(&mut self) {
        self.last_phase = Instant::now();
    }
    pub fn terminating(&mut self) {
        self.state = HealthState::Terminating;
    }
    pub fn cleaned(&mut self) {
        self.state = HealthState::Cleaned;
    }

    pub fn update(&mut self, sample: ResourceSample, cdp_healthy: bool) -> HealthSnapshot {
        let mut reasons = Vec::new();
        if sample.memory_current_bytes >= self.memory_limit {
            reasons.push("memory-limit".into());
        }
        if sample.process_count >= self.process_limit {
            reasons.push("process-limit".into());
        }
        if sample.file_descriptor_count > self.process_limit.saturating_mul(64) {
            reasons.push("file-descriptor-growth".into());
        }
        if sample.memory_pressure_some_avg10.unwrap_or(0.0) >= 10.0 {
            reasons.push("memory-pressure".into());
        }
        if cdp_healthy {
            self.consecutive_cdp_failures = 0;
        } else {
            self.consecutive_cdp_failures = self.consecutive_cdp_failures.saturating_add(1);
        }
        if self.consecutive_cdp_failures >= 3 {
            reasons.push("cdp-unresponsive".into());
        }
        if self.last_phase.elapsed() >= self.phase_timeout {
            reasons.push("phase-timeout".into());
        }

        if reasons
            .iter()
            .any(|r| r == "cdp-unresponsive" || r == "phase-timeout")
        {
            self.state = HealthState::Stuck;
        } else if reasons.is_empty() {
            self.consecutive_degraded = 0;
            self.state = HealthState::Healthy;
        } else {
            self.consecutive_degraded = self.consecutive_degraded.saturating_add(1);
            self.state = if self.consecutive_degraded >= 3 {
                HealthState::Degraded
            } else {
                HealthState::Healthy
            };
        }
        HealthSnapshot {
            state: self.state,
            reasons,
            resources: sample,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn degradation_requires_hysteresis() {
        let mut h = HealthMonitor::new(100, 10, Duration::from_secs(60));
        let sample = ResourceSample {
            memory_current_bytes: 101,
            ..Default::default()
        };
        assert_eq!(h.update(sample.clone(), true).state, HealthState::Healthy);
        assert_eq!(h.update(sample.clone(), true).state, HealthState::Healthy);
        assert_eq!(h.update(sample, true).state, HealthState::Degraded);
    }
    #[test]
    fn repeated_cdp_failure_is_stuck() {
        let mut h = HealthMonitor::new(100, 10, Duration::from_secs(60));
        assert_eq!(
            h.update(ResourceSample::default(), false).state,
            HealthState::Healthy
        );
        assert_eq!(
            h.update(ResourceSample::default(), false).state,
            HealthState::Healthy
        );
        assert_eq!(
            h.update(ResourceSample::default(), false).state,
            HealthState::Stuck
        );
    }
}
