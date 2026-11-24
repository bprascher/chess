//! DD-EN-013: typed failures and their mapping to the architecture 5.5 `EngineUnavailable`
//! causes. `Display` names the category only: never a path, a command line or engine output
//! (SWE-001 section 6, ERR row; NFR-10). Details go to `tracing` with the instance id.

use crate::types::SearchResult;

/// How the application layer must treat an error (SDD 3.6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    /// `EngineUnavailable{cause}` with this catalogue cause string.
    Unavailable(&'static str),
    /// Not an engine failure (background analysis yielded to an interactive request).
    NotAFailure,
    /// `Internal{id}`: a defect, not an engine outage.
    Internal,
}

#[derive(Clone, PartialEq, Eq, thiserror::Error)]
pub enum EngineError {
    #[error("engine binary is missing")]
    BinaryMissing,
    #[error("engine binary is not executable or has unsafe permissions")]
    NotExecutable,
    #[error("engine override path is invalid")]
    InvalidOverride,
    #[error("engine process could not be started")]
    SpawnFailed,
    #[error("engine did not answer the handshake in time")]
    HandshakeTimeout,
    /// `found` is engine output; it is for logs and is not shown by `Display`.
    #[error("engine binary reports the wrong version")]
    WrongVersion { found: String },
    #[error("engine process exited")]
    ProcessExited { code: Option<i32> },
    #[error("engine search timed out")]
    SearchTimeout,
    #[error("engine produced unusable output")]
    BadOutput,
    #[error("background analysis was preempted")]
    Preempted { partial: Option<Box<SearchResult>> },
    #[error("personality needs an option the engine does not offer or has a value out of range")]
    InvalidPersonality { option: String },
    #[error("invalid search request")]
    InvalidRequest { reason: &'static str },
    // Additive internal variants (choice F-E5): not in the SDD table, all `Internal`.
    #[error("invalid engine configuration")]
    InvalidConfig { field: &'static str },
    #[error("engine instance is busy with another search")]
    Busy,
    #[error("engine instance is shut down")]
    Closed,
}

/// `Debug` is written by hand (EF-14): `WrongVersion.found` and `InvalidPersonality.option` hold
/// text the engine sent, and `{:?}` of an error must never put it in a log. Only the length is
/// shown. Everything else prints as the derive would.
impl std::fmt::Debug for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::BinaryMissing => f.write_str("BinaryMissing"),
            EngineError::NotExecutable => f.write_str("NotExecutable"),
            EngineError::InvalidOverride => f.write_str("InvalidOverride"),
            EngineError::SpawnFailed => f.write_str("SpawnFailed"),
            EngineError::HandshakeTimeout => f.write_str("HandshakeTimeout"),
            EngineError::WrongVersion { found } => f
                .debug_struct("WrongVersion")
                .field(
                    "found",
                    &format_args!("<{} chars of engine text>", found.chars().count()),
                )
                .finish(),
            EngineError::ProcessExited { code } => {
                f.debug_struct("ProcessExited").field("code", code).finish()
            }
            EngineError::SearchTimeout => f.write_str("SearchTimeout"),
            EngineError::BadOutput => f.write_str("BadOutput"),
            EngineError::Preempted { partial } => f
                .debug_struct("Preempted")
                .field("partial", partial)
                .finish(),
            EngineError::InvalidPersonality { option } => f
                .debug_struct("InvalidPersonality")
                .field(
                    "option",
                    &format_args!("<{} chars of engine text>", option.chars().count()),
                )
                .finish(),
            EngineError::InvalidRequest { reason } => f
                .debug_struct("InvalidRequest")
                .field("reason", reason)
                .finish(),
            EngineError::InvalidConfig { field } => f
                .debug_struct("InvalidConfig")
                .field("field", field)
                .finish(),
            EngineError::Busy => f.write_str("Busy"),
            EngineError::Closed => f.write_str("Closed"),
        }
    }
}

impl EngineError {
    /// The documented mapping to architecture 5.5 (CT-EN-060).
    pub fn class(&self) -> ErrorClass {
        match self {
            EngineError::BinaryMissing
            | EngineError::NotExecutable
            | EngineError::InvalidOverride => ErrorClass::Unavailable("binary_missing"),
            EngineError::SpawnFailed => ErrorClass::Unavailable("spawn_failed"),
            EngineError::HandshakeTimeout => ErrorClass::Unavailable("handshake_timeout"),
            EngineError::WrongVersion { .. } => ErrorClass::Unavailable("wrong_version"),
            EngineError::ProcessExited { .. } => ErrorClass::Unavailable("crashed"),
            EngineError::SearchTimeout => ErrorClass::Unavailable("timeout"),
            EngineError::BadOutput => ErrorClass::Unavailable("bad_output"),
            EngineError::Preempted { .. } => ErrorClass::NotAFailure,
            EngineError::InvalidPersonality { .. }
            | EngineError::InvalidRequest { .. }
            | EngineError::InvalidConfig { .. }
            | EngineError::Busy
            | EngineError::Closed => ErrorClass::Internal,
        }
    }
}
