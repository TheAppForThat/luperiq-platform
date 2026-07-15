use serde::{Deserialize, Serialize};

/// A user identity with roles.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identity {
    pub id: String,
    pub roles: Vec<String>,
}

/// Status of a scheduled task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Completed,
    Failed,
}

/// A durably-scheduled task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScheduledTask {
    pub task_id: String,
    pub payload: Vec<u8>,
    pub run_at: u64,
    pub status: TaskStatus,
    pub error: Option<String>,
}

/// Payload for a module enable/disable state change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleStatePayload {
    pub enabled: bool,
    pub changed_at: u64,
}
