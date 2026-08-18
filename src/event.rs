use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::metrics::ResourceSample;
use crate::policy::Mechanism;
use crate::protocol::Phase;

pub const EVENT_SCHEMA_VERSION: u16 = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub schema_version: u16,
    pub wall_time_unix_ms: u128,
    pub monotonic_ns: u128,
    pub session_id: String,
    pub generation: u64,
    pub phase: Phase,
    pub mechanism: Mechanism,
    #[serde(flatten)]
    pub kind: EventKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum EventKind {
    PhaseChanged,
    PolicyApplied {
        action: String,
    },
    ResourceSample(ResourceSample),
    TaskFinished {
        workload: String,
        wait_ms: u64,
        #[serde(default = "default_concurrency")]
        concurrency: usize,
        #[serde(default)]
        block_id: u64,
        success: bool,
        latency_ms: f64,
        browser_cpu_usec: u64,
        #[serde(default)]
        browser_wait_cpu_usec: u64,
        #[serde(default)]
        browser_cpu_throttled_usec: u64,
        #[serde(default)]
        browser_memory_current_bytes: u64,
        #[serde(default)]
        browser_memory_peak_bytes: Option<u64>,
        #[serde(default)]
        browser_io_read_bytes: u64,
        #[serde(default)]
        browser_io_write_bytes: u64,
        #[serde(default)]
        browser_cpu_pressure_some_avg10: Option<f64>,
        #[serde(default)]
        browser_memory_pressure_some_avg10: Option<f64>,
        #[serde(default)]
        browser_io_pressure_some_avg10: Option<f64>,
        driver_cpu_usec: u64,
        governor_cpu_usec: u64,
        server_cpu_usec: u64,
        host_steal_ticks: u64,
        resume_latency_ms: f64,
        reconnects: u64,
        sequence_gaps: u64,
        #[serde(default)]
        background_operations: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        failure: Option<String>,
    },
}

fn default_concurrency() -> usize {
    1
}

#[derive(Clone)]
pub struct EventWriter {
    inner: Arc<Mutex<BufWriter<File>>>,
    started: Instant,
}

impl EventWriter {
    pub fn create(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        let file = File::create(path).with_context(|| format!("creating {}", path.display()))?;
        Ok(Self {
            inner: Arc::new(Mutex::new(BufWriter::new(file))),
            started: Instant::now(),
        })
    }

    pub fn write(
        &self,
        session_id: &str,
        generation: u64,
        phase: Phase,
        mechanism: &Mechanism,
        kind: EventKind,
    ) -> Result<()> {
        let event = Event {
            schema_version: EVENT_SCHEMA_VERSION,
            wall_time_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            monotonic_ns: self.started.elapsed().as_nanos(),
            session_id: session_id.to_owned(),
            generation,
            phase,
            mechanism: mechanism.clone(),
            kind,
        };
        let mut writer = self
            .inner
            .lock()
            .map_err(|_| anyhow::anyhow!("event writer poisoned"))?;
        serde_json::to_writer(&mut *writer, &event)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        Ok(())
    }
}
