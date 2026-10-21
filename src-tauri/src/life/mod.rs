pub mod crypto;
pub mod record;
pub mod schema;
pub mod store;

pub use crypto::{CryptoError, LifeCrypto};
pub use record::{
    Assumption, AssumptionStatus, AuditEntry, DataClass, GRADE_0_WORD_ONLY,
    GRADE_1_CHECKABLE_DETAIL, GRADE_2_OBSERVED, GRADE_3_INDEPENDENT, NewRecord, Plan, PlanVersion,
    Record, RecordKind, RecordSource, validate_source_grade,
};
pub use schema::{Opened, VERSION as SCHEMA_VERSION};
pub use store::{ExportSummary, LifeStore};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LifeError {
    #[error("the life database is written by a newer version (schema {0})")]
    NewerVersion(i64),

    #[error("sqlite error: {0}")]
    Sqlite(String),

    #[error("invalid data class: {0}")]
    InvalidDataClass(String),

    #[error("invalid record kind: {0}")]
    InvalidRecordKind(String),

    #[error("invalid record source: {0}")]
    InvalidRecordSource(String),

    #[error("invalid assumption status: {0}")]
    InvalidAssumptionStatus(String),

    #[error("invalid evidence grade: {0} (must be 0..=3)")]
    InvalidGrade(u8),

    #[error("inconsistent source and grade: {record_source} cannot have grade {grade}: {reason}")]
    InconsistentSourceGrade {
        record_source: RecordSource,
        grade: u8,
        reason: &'static str,
    },

    #[error("consent denied for data class '{0}'")]
    ClassConsentDenied(DataClass),

    #[error("life module is disabled")]
    ModuleDisabled,

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("record not found: {0}")]
    RecordNotFound(String),

    #[error("plan not found: {0}")]
    PlanNotFound(String),

    #[error("io error: {0}")]
    Io(String),

    #[error("endpoint transmission rejected: {0}")]
    SendDenied(String),
}

impl From<rusqlite::Error> for LifeError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Sqlite(err.to_string())
    }
}

impl From<crypto::CryptoError> for LifeError {
    fn from(err: crypto::CryptoError) -> Self {
        Self::Crypto(err.to_string())
    }
}

impl From<std::io::Error> for LifeError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}
