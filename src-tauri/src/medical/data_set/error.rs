//! Why a medical data set was refused, typed so the frontend can explain it
//! in the user's language (manual install from downloaded files). Written
//! with Claude Code.

use serde::Serialize;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Serialize, thiserror::Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SetError {
    #[error("{name} is missing")]
    MissingFile { name: String },
    #[error("Bad signature: {detail}")]
    BadSignature { detail: String },
    #[error("Invalid manifest: {detail}")]
    InvalidManifest { detail: String },
    #[error("Data set for model \"{model}\", this app uses \"{expected}\"")]
    WrongModel { model: String, expected: String },
    #[error("Data set needs Plumed {min_app_version} or newer")]
    AppTooOld { min_app_version: String },
    #[error("Version {version} is not newer than the data in use ({active})")]
    NotNewer { version: String, active: String },
    #[error("{name} does not match the manifest")]
    FileMismatch { name: String },
    #[error("{detail}")]
    Other { detail: String },
}

impl SetError {
    pub fn invalid(detail: impl Display) -> Self {
        Self::InvalidManifest {
            detail: detail.to_string(),
        }
    }

    pub fn other(detail: impl Display) -> Self {
        Self::Other {
            detail: detail.to_string(),
        }
    }
}

impl From<String> for SetError {
    fn from(detail: String) -> Self {
        Self::Other { detail }
    }
}

impl From<&str> for SetError {
    fn from(detail: &str) -> Self {
        Self::other(detail)
    }
}

impl From<SetError> for String {
    fn from(error: SetError) -> Self {
        error.to_string()
    }
}
