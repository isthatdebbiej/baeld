use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::policy::Mechanism;

fn one() -> u16 {
    1
}
fn chromium() -> PathBuf {
    ".baeld/chromium".into()
}
fn adaptive() -> PolicyMode {
    PolicyMode::Adaptive
}
fn required() -> FreezeCompatibility {
    FreezeCompatibility::Required
}
fn ephemeral() -> SessionMode {
    SessionMode::Ephemeral
}
fn memory() -> u64 {
    2_048
}
fn processes() -> u64 {
    128
}
fn shutdown() -> u64 {
    5_000
}
fn safe_filter() -> FilterProfile {
    FilterProfile::Safe
}
fn nav_limit() -> usize {
    8
}
fn action_limit() -> usize {
    12
}
fn stagger() -> u64 {
    100
}
fn cpu_pressure() -> f64 {
    20.0
}
fn memory_pressure() -> f64 {
    10.0
}
fn sample_ms() -> u64 {
    500
}
fn phase_timeout_ms() -> u64 {
    120_000
}
fn true_value() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    #[serde(default = "one")]
    pub schema_version: u16,
    #[serde(default = "chromium")]
    pub chromium: PathBuf,
    #[serde(default)]
    pub chrome_args: Vec<String>,
    #[serde(default)]
    pub allow_extensions: bool,
    #[serde(default)]
    pub policy: PolicyConfig,
    #[serde(default)]
    pub session: SessionConfig,
    #[serde(default)]
    pub filtering: FilteringConfig,
    #[serde(default)]
    pub concurrency: ConcurrencyConfig,
    #[serde(default)]
    pub telemetry: TelemetryConfig,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            schema_version: one(),
            chromium: chromium(),
            chrome_args: vec![],
            allow_extensions: false,
            policy: PolicyConfig::default(),
            session: SessionConfig::default(),
            filtering: FilteringConfig::default(),
            concurrency: ConcurrencyConfig::default(),
            telemetry: TelemetryConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PolicyMode {
    Observe,
    Safe,
    Adaptive,
    Explicit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FreezeCompatibility {
    Required,
    Confirmed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyConfig {
    #[serde(default = "adaptive")]
    pub mode: PolicyMode,
    #[serde(default = "required")]
    pub freeze_compatibility: FreezeCompatibility,
    #[serde(default)]
    pub compatibility_file: Option<PathBuf>,
    #[serde(default)]
    pub explicit: Option<ExplicitPolicy>,
    #[serde(default = "sample_ms")]
    pub sample_ms: u64,
    #[serde(default = "phase_timeout_ms")]
    pub phase_timeout_ms: u64,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            mode: adaptive(),
            freeze_compatibility: required(),
            compatibility_file: None,
            explicit: None,
            sample_ms: sample_ms(),
            phase_timeout_ms: phase_timeout_ms(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExplicitPolicy {
    pub mechanism: String,
    #[serde(default)]
    pub delay_ms: Option<u64>,
    #[serde(default)]
    pub quota_us: Option<u64>,
    #[serde(default)]
    pub period_us: Option<u64>,
}

impl ExplicitPolicy {
    pub fn mechanism(&self) -> Result<Mechanism> {
        let mechanism = match self.mechanism.as_str() {
            "baseline" => Mechanism::Baseline,
            "chrome-lifecycle-freeze" => Mechanism::ChromeLifecycleFreeze,
            "cgroup-freeze" => Mechanism::CgroupFreeze {
                delay_ms: self.delay_ms.unwrap_or(500),
            },
            "cpu-quota" => Mechanism::CpuQuota {
                quota_us: self.quota_us.context("cpu-quota requires quota_us")?,
                period_us: self.period_us.unwrap_or(100_000),
            },
            other => bail!("unknown explicit mechanism {other:?}"),
        };
        mechanism.validate()?;
        Ok(mechanism)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionMode {
    Ephemeral,
    Persistent,
    Warm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionConfig {
    #[serde(default = "ephemeral")]
    pub mode: SessionMode,
    pub profile_dir: Option<PathBuf>,
    #[serde(default = "memory")]
    pub max_memory_mb: u64,
    #[serde(default = "processes")]
    pub max_processes: u64,
    #[serde(default = "shutdown")]
    pub shutdown_grace_ms: u64,
    #[serde(default)]
    pub recycle_on_degraded: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            mode: ephemeral(),
            profile_dir: None,
            max_memory_mb: memory(),
            max_processes: processes(),
            shutdown_grace_ms: shutdown(),
            recycle_on_degraded: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FilterProfile {
    Safe,
    Balanced,
    Text,
    Visual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilteringConfig {
    #[serde(default = "safe_filter")]
    pub profile: FilterProfile,
}
impl Default for FilteringConfig {
    fn default() -> Self {
        Self {
            profile: safe_filter(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConcurrencyConfig {
    #[serde(default = "nav_limit")]
    pub max_active_navigation: usize,
    #[serde(default = "action_limit")]
    pub max_active_actions: usize,
    #[serde(default = "stagger")]
    pub stagger_resume_ms: u64,
    #[serde(default = "cpu_pressure")]
    pub max_cpu_pressure_avg10: f64,
    #[serde(default = "memory_pressure")]
    pub max_memory_pressure_avg10: f64,
}
impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            max_active_navigation: nav_limit(),
            max_active_actions: action_limit(),
            stagger_resume_ms: stagger(),
            max_cpu_pressure_avg10: cpu_pressure(),
            max_memory_pressure_avg10: memory_pressure(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TelemetryConfig {
    #[serde(default = "true_value")]
    pub jsonl: bool,
    #[serde(default)]
    pub otel_endpoint: String,
}
impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            jsonl: true,
            otel_endpoint: String::new(),
        }
    }
}

impl RuntimeConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let raw =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let config: Self =
            toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            bail!("unsupported configuration schema {}", self.schema_version);
        }
        if self.concurrency.max_active_navigation == 0 || self.concurrency.max_active_actions == 0 {
            bail!("concurrency limits must be positive");
        }
        if self.concurrency.max_cpu_pressure_avg10 <= 0.0
            || self.concurrency.max_memory_pressure_avg10 <= 0.0
        {
            bail!("pressure thresholds must be positive");
        }
        if self.policy.sample_ms < 100 {
            bail!("policy.sample_ms must be at least 100");
        }
        if self.session.max_memory_mb < 128 || self.session.max_processes < 8 {
            bail!("session limits are too small for Chromium");
        }
        if self.session.mode == SessionMode::Persistent && self.session.profile_dir.is_none() {
            bail!("persistent sessions require session.profile_dir");
        }
        if self.policy.mode == PolicyMode::Explicit {
            let mechanism = self
                .policy
                .explicit
                .as_ref()
                .context("explicit policy mode requires policy.explicit")?
                .mechanism()?;
            if mechanism == Mechanism::ChromeLifecycleFreeze {
                bail!("chrome lifecycle freeze is benchmark-only until a page-bound runtime adapter is configured");
            }
        }
        if self.policy.freeze_compatibility == FreezeCompatibility::Confirmed
            && self.policy.compatibility_file.is_none()
        {
            bail!("confirmed freeze compatibility requires policy.compatibility_file");
        }
        if !self.telemetry.otel_endpoint.is_empty()
            && !self.telemetry.otel_endpoint.starts_with("unix://")
        {
            bail!("telemetry.otel_endpoint must be empty or a local unix:// socket");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_are_safe_to_parse() {
        RuntimeConfig::default().validate().unwrap();
    }
    #[test]
    fn explicit_requires_mechanism() {
        let mut c = RuntimeConfig::default();
        c.policy.mode = PolicyMode::Explicit;
        assert!(c.validate().is_err());
    }
    #[test]
    fn persistent_requires_profile() {
        let mut c = RuntimeConfig::default();
        c.session.mode = SessionMode::Persistent;
        assert!(c.validate().is_err());
    }

    #[test]
    fn confirmed_compatibility_requires_records() {
        let mut c = RuntimeConfig::default();
        c.policy.freeze_compatibility = FreezeCompatibility::Confirmed;
        assert!(c.validate().is_err());
    }

    #[test]
    fn lifecycle_freeze_is_not_silently_accepted_by_runtime() {
        let mut c = RuntimeConfig::default();
        c.policy.mode = PolicyMode::Explicit;
        c.policy.explicit = Some(ExplicitPolicy {
            mechanism: "chrome-lifecycle-freeze".into(),
            delay_ms: None,
            quota_us: None,
            period_us: None,
        });
        assert!(c.validate().is_err());
    }
}
