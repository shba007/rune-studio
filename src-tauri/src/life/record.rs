use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use super::LifeError;

/// Grade 0: Your word only ("I went to the gym").
pub const GRADE_0_WORD_ONLY: u8 = 0;

/// Grade 1: Your word with a specific checkable detail ("Back and biceps, 50 min, ended 7:10").
pub const GRADE_1_CHECKABLE_DETAIL: u8 = 1;

/// Grade 2: Observed by a tool or imported data the user did not type (commits in repo, wearable export).
pub const GRADE_2_OBSERVED: u8 = 2;

/// Grade 3: Independent and hard to fake (bank statement, measured result over time).
pub const GRADE_3_INDEPENDENT: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataClass {
    General,
    Health,
    Mind,
    Finance,
}

impl DataClass {
    pub fn all() -> &'static [DataClass] {
        &[
            DataClass::General,
            DataClass::Health,
            DataClass::Mind,
            DataClass::Finance,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DataClass::General => "general",
            DataClass::Health => "health",
            DataClass::Mind => "mind",
            DataClass::Finance => "finance",
        }
    }

    /// Whether this data class contains sensitive personal information
    /// and therefore MUST be strictly local-only and encrypted at rest.
    pub fn is_sensitive(&self) -> bool {
        match self {
            DataClass::General => false,
            DataClass::Health | DataClass::Mind | DataClass::Finance => true,
        }
    }
}

impl fmt::Display for DataClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for DataClass {
    type Err = LifeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "general" => Ok(DataClass::General),
            "health" => Ok(DataClass::Health),
            "mind" => Ok(DataClass::Mind),
            "finance" => Ok(DataClass::Finance),
            other => Err(LifeError::InvalidDataClass(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordKind {
    Claim,
    Fact,
    Preference,
    Note,
    Event,
}

impl RecordKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecordKind::Claim => "claim",
            RecordKind::Fact => "fact",
            RecordKind::Preference => "preference",
            RecordKind::Note => "note",
            RecordKind::Event => "event",
        }
    }
}

impl fmt::Display for RecordKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for RecordKind {
    type Err = LifeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "claim" => Ok(RecordKind::Claim),
            "fact" => Ok(RecordKind::Fact),
            "preference" => Ok(RecordKind::Preference),
            "note" => Ok(RecordKind::Note),
            "event" => Ok(RecordKind::Event),
            other => Err(LifeError::InvalidRecordKind(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordSource {
    UserSaid,
    UserDetail,
    Tool,
    Import,
}

impl RecordSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecordSource::UserSaid => "user_said",
            RecordSource::UserDetail => "user_detail",
            RecordSource::Tool => "tool",
            RecordSource::Import => "import",
        }
    }
}

impl fmt::Display for RecordSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for RecordSource {
    type Err = LifeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "user_said" => Ok(RecordSource::UserSaid),
            "user_detail" => Ok(RecordSource::UserDetail),
            "tool" => Ok(RecordSource::Tool),
            "import" => Ok(RecordSource::Import),
            other => Err(LifeError::InvalidRecordSource(other.to_string())),
        }
    }
}

/// Checks that a record's source and grade are consistent.
/// - a `user_said` record can never be grade 2 or 3
/// - a `user_detail` record can never be grade 2 or 3
/// - a `tool` or `import` record cannot be grade 0
/// - grade must be in range 0..=3
pub fn validate_source_grade(source: &RecordSource, grade: u8) -> Result<(), LifeError> {
    if grade > 3 {
        return Err(LifeError::InvalidGrade(grade));
    }
    match source {
        RecordSource::UserSaid | RecordSource::UserDetail => {
            if grade >= 2 {
                return Err(LifeError::InconsistentSourceGrade {
                    record_source: *source,
                    grade,
                    reason: "user-typed claims cannot claim tool-observed (E2) or independent (E3) grades",
                });
            }
        }
        RecordSource::Tool | RecordSource::Import => {
            if grade == 0 {
                return Err(LifeError::InconsistentSourceGrade {
                    record_source: *source,
                    grade,
                    reason: "tool observations and imports must be supported by checkable data (grade >= 1)",
                });
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssumptionStatus {
    Untested,
    Supported,
    Refuted,
}

impl AssumptionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssumptionStatus::Untested => "untested",
            AssumptionStatus::Supported => "supported",
            AssumptionStatus::Refuted => "refuted",
        }
    }
}

impl fmt::Display for AssumptionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for AssumptionStatus {
    type Err = LifeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "untested" => Ok(AssumptionStatus::Untested),
            "supported" => Ok(AssumptionStatus::Supported),
            "refuted" => Ok(AssumptionStatus::Refuted),
            other => Err(LifeError::InvalidAssumptionStatus(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub class: DataClass,
    pub kind: RecordKind,
    pub body: String,
    pub source: RecordSource,
    pub grade: u8,
    pub occurred_utc: String,
    pub created_utc: String,
    pub supports_json: Option<String>,
    pub deleted: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewRecord {
    pub id: Option<String>,
    pub class: DataClass,
    pub kind: RecordKind,
    pub body: String,
    pub source: RecordSource,
    pub grade: u8,
    pub occurred_utc: Option<String>,
    pub supports_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assumption {
    pub id: String,
    pub text: String,
    pub class: DataClass,
    pub status: AssumptionStatus,
    pub evidence_json: Option<String>,
    pub created_utc: String,
    pub changed_utc: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub current_version: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanVersion {
    pub id: String,
    pub version: i64,
    pub body_json: String,
    pub reason: Option<String>,
    pub evidence_json: Option<String>,
    pub created_utc: String,
    pub approved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: i64,
    pub at_utc: String,
    pub actor: String,
    pub action: String,
    pub subject: String,
    pub detail: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_and_grade_consistency_checks() {
        assert!(validate_source_grade(&RecordSource::UserSaid, 0).is_ok());
        assert!(validate_source_grade(&RecordSource::UserSaid, 1).is_ok());
        assert!(validate_source_grade(&RecordSource::UserSaid, 2).is_err());
        assert!(validate_source_grade(&RecordSource::UserSaid, 3).is_err());

        assert!(validate_source_grade(&RecordSource::UserDetail, 0).is_ok());
        assert!(validate_source_grade(&RecordSource::UserDetail, 1).is_ok());
        assert!(validate_source_grade(&RecordSource::UserDetail, 2).is_err());

        assert!(validate_source_grade(&RecordSource::Tool, 0).is_err());
        assert!(validate_source_grade(&RecordSource::Tool, 1).is_ok());
        assert!(validate_source_grade(&RecordSource::Tool, 2).is_ok());
        assert!(validate_source_grade(&RecordSource::Tool, 3).is_ok());

        assert!(validate_source_grade(&RecordSource::Import, 0).is_err());
        assert!(validate_source_grade(&RecordSource::Import, 2).is_ok());

        assert!(validate_source_grade(&RecordSource::UserSaid, 4).is_err());
    }

    #[test]
    fn sensitive_classes_identification() {
        assert!(!DataClass::General.is_sensitive());
        assert!(DataClass::Health.is_sensitive());
        assert!(DataClass::Mind.is_sensitive());
        assert!(DataClass::Finance.is_sensitive());
    }

    #[test]
    fn roundtrip_enums() {
        assert_eq!("health".parse::<DataClass>().unwrap(), DataClass::Health);
        assert_eq!("claim".parse::<RecordKind>().unwrap(), RecordKind::Claim);
        assert_eq!(
            "user_said".parse::<RecordSource>().unwrap(),
            RecordSource::UserSaid
        );
        assert_eq!(
            "supported".parse::<AssumptionStatus>().unwrap(),
            AssumptionStatus::Supported
        );
    }
}
