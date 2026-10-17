use serde::Serialize;
use std::fmt;

/// Error type returned to the frontend. Serialized as `{ "kind": "...", ... }`.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AppError {
    Validation {
        field: &'static str,
        message: String,
    },
    NotFound {
        entity: &'static str,
        id: i64,
    },
    Database {
        message: String,
    },
    Internal {
        message: String,
    },
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn validation(field: &'static str, message: impl Into<String>) -> Self {
        AppError::Validation {
            field,
            message: message.into(),
        }
    }

    pub fn not_found(entity: &'static str, id: i64) -> Self {
        AppError::NotFound { entity, id }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        AppError::Internal {
            message: message.into(),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Validation { field, message } => write!(f, "invalid {field}: {message}"),
            AppError::NotFound { entity, id } => write!(f, "{entity} {id} not found"),
            AppError::Database { message } => write!(f, "database error: {message}"),
            AppError::Internal { message } => write!(f, "internal error: {message}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Database {
            message: e.to_string(),
        }
    }
}
