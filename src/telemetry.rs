use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

use crate::health::HealthSnapshot;
use crate::protocol::Phase;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeEvent {
    pub schema_version: u16,
    pub timestamp_unix_ms: u128,
    pub session_id: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<Phase>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<HealthSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone)]
pub struct Telemetry {
    file: Option<Arc<Mutex<File>>>,
    endpoint: Option<String>,
}

impl Telemetry {
    pub fn new(path: &Path, jsonl: bool, endpoint: &str) -> Result<Self> {
        let file = if jsonl {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let file = OpenOptions::new().create(true).append(true).open(path)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(fs::Permissions::from_mode(0o600))?;
            }
            Some(Arc::new(Mutex::new(file)))
        } else {
            None
        };
        Ok(Self {
            file,
            endpoint: (!endpoint.is_empty()).then(|| endpoint.trim_end_matches('/').to_owned()),
        })
    }

    pub async fn emit(&self, mut event: RuntimeEvent) {
        event.timestamp_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|v| v.as_millis())
            .unwrap_or(0);
        if let Some(file) = &self.file {
            if let Ok(mut file) = file.lock() {
                let _ = writeln!(
                    file,
                    "{}",
                    serde_json::to_string(&event).unwrap_or_default()
                );
            }
        }
        if let Some(endpoint) = &self.endpoint {
            let body = json!({"resourceLogs":[{"resource":{"attributes":[{"key":"service.name","value":{"stringValue":"baeld"}}]},"scopeLogs":[{"scope":{"name":"baeld.runtime"},"logRecords":[{"timeUnixNano":(event.timestamp_unix_ms * 1_000_000).to_string(),"severityText":"INFO","body":{"stringValue":serde_json::to_string(&event).unwrap_or_default()},"attributes":[{"key":"baeld.session_id","value":{"stringValue":event.session_id}}]}]}]}]});
            let _ =
                tokio::time::timeout(Duration::from_millis(250), send_otlp(endpoint, &body)).await;
        }
    }
}

async fn send_otlp(endpoint: &str, body: &serde_json::Value) -> Result<()> {
    let socket = endpoint
        .strip_prefix("unix://")
        .ok_or_else(|| anyhow::anyhow!("otel_endpoint must be a local unix:// collector socket"))?;
    let payload = serde_json::to_vec(body)?;
    let mut stream = UnixStream::connect(socket).await?;
    let header = format!(
        "POST /v1/logs HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(&payload).await?;
    stream.shutdown().await?;
    Ok(())
}

impl RuntimeEvent {
    pub fn new(session_id: &str, kind: &str) -> Self {
        Self {
            schema_version: 1,
            timestamp_unix_ms: 0,
            session_id: session_id.into(),
            kind: kind.into(),
            phase: None,
            action: None,
            health: None,
            message: None,
        }
    }
}
