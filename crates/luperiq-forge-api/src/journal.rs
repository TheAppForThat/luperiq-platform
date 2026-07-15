use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during journal operations.
#[derive(Debug, Error)]
pub enum ForgeError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Encode(String),
    #[error("deserialization error: {0}")]
    Decode(String),
    #[error("checksum mismatch while replaying WAL")]
    ChecksumMismatch,
    #[error("signature verification failed")]
    SignatureMismatch,
    #[error("merkle root verification failed")]
    MerkleRootMismatch,
    #[error("unexpected WAL truncation")]
    TruncatedWAL,
    #[error("payload encryption error: {0}")]
    Encryption(String),
}

/// Statistics about the current journal state.
#[derive(Debug, Clone)]
pub struct JournalStats {
    pub aggregates: usize,
    pub events: u64,
    pub merkle_root: [u8; 32],
    pub wal_path: PathBuf,
    pub snapshot_path: PathBuf,
}
