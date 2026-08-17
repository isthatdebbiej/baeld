use std::fmt;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Mechanism {
    Baseline,
    ChromeLifecycleFreeze,
    CpuQuota { quota_us: u64, period_us: u64 },
    CgroupFreeze { delay_ms: u64 },
}

impl Mechanism {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::CpuQuota {
                quota_us,
                period_us,
            } => {
                if *period_us == 0 || *quota_us == 0 || quota_us > period_us {
                    bail!("CPU quota must satisfy 0 < quota_us <= period_us");
                }
            }
            Self::CgroupFreeze { .. } | Self::Baseline | Self::ChromeLifecycleFreeze => {}
        }
        Ok(())
    }

    pub fn slug(&self) -> String {
        match self {
            Self::Baseline => "baseline".into(),
            Self::ChromeLifecycleFreeze => "chrome-lifecycle-freeze".into(),
            Self::CpuQuota {
                quota_us,
                period_us,
            } => {
                format!("cpu-quota-{quota_us}-{period_us}")
            }
            Self::CgroupFreeze { delay_ms } => format!("cgroup-freeze-{delay_ms}ms"),
        }
    }
}

impl fmt::Display for Mechanism {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.slug())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_quota() {
        assert!(Mechanism::CpuQuota {
            quota_us: 101,
            period_us: 100,
        }
        .validate()
        .is_err());
    }

    #[test]
    fn serializes_stable_wire_shape() {
        let json = serde_json::to_string(&Mechanism::CgroupFreeze { delay_ms: 500 }).unwrap();
        assert_eq!(json, r#"{"kind":"cgroup-freeze","delay_ms":500}"#);
    }
}
