use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Starting,
    Navigating,
    Observing,
    WaitingForModel,
    Acting,
    Verifying,
    Settling,
    Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseRequest {
    pub schema_version: u16,
    pub session_id: String,
    pub generation: u64,
    pub phase: Phase,
    #[serde(default)]
    pub expected_wait_ms: Option<u64>,
    #[serde(default)]
    pub page_cdp_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseAck {
    pub schema_version: u16,
    pub session_id: String,
    pub generation: u64,
    pub accepted: bool,
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl PhaseRequest {
    pub fn validate(&self, expected_session: &str, last_generation: u64) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(format!(
                "unsupported schema version {}",
                self.schema_version
            ));
        }
        if self.session_id != expected_session {
            return Err("session id mismatch".into());
        }
        if self.generation <= last_generation {
            return Err(format!(
                "generation {} is not newer than {}",
                self.generation, last_generation
            ));
        }
        if self.phase == Phase::WaitingForModel && self.expected_wait_ms.is_none() {
            return Err("waiting_for_model requires expected_wait_ms".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_stale_generation() {
        let req = PhaseRequest {
            schema_version: 1,
            session_id: "s".into(),
            generation: 4,
            phase: Phase::Acting,
            expected_wait_ms: None,
            page_cdp_session_id: None,
        };
        assert!(req.validate("s", 4).is_err());
    }
}
