use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Identifies what initiated a change — human edit, AI workflow, system action, etc.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ChangeSource {
    #[default]
    Legacy,
    Human,
    AiWorkflow,
    System,
    Import,
}

/// Actor metadata for an event. Stored inside the payload as a JSON wrapper
/// rather than as struct fields, so existing WAL files remain compatible.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventActor {
    pub actor_id: String,
    pub change_source: ChangeSource,
    pub change_reason: String,
}

/// Core event stored in the Write-Ahead Log.
///
/// The struct layout is unchanged from the original to maintain binary
/// compatibility with existing WAL files. Actor tracking metadata is
/// stored separately via the [`EventActor`] companion and persisted
/// alongside events in a sidecar aggregate (`ChangeLog:*`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApexEvent {
    pub id: Ulid,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub version: u64,
    pub timestamp_ns: u64,
    pub payload: Vec<u8>,
    pub signature: [u8; 32],
    pub merkle_root: [u8; 32],
}

impl ApexEvent {
    pub fn new(
        aggregate_type: impl Into<String>,
        aggregate_id: impl Into<String>,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            id: Ulid::new(),
            aggregate_type: aggregate_type.into(),
            aggregate_id: aggregate_id.into(),
            version: 0,
            timestamp_ns: now_ns(),
            payload,
            signature: [0; 32],
            merkle_root: [0; 32],
        }
    }

    pub fn key(&self) -> String {
        format!("{}:{}", self.aggregate_type, self.aggregate_id)
    }
}

fn now_ns() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}
