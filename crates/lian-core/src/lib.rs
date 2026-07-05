//! LIAN core: local-first domain logic over SQLite.
//!
//! This crate is deliberately independent of Tauri so the entire domain layer
//! (schema, contracts, assessment engines, analysis, backup/export) is
//! testable headlessly. The desktop shell forwards typed API calls to
//! [`api::dispatch`].

pub mod analysis;
pub mod api;
pub mod assessments;
pub mod backup;
pub mod db;
pub mod jsonq;
pub mod reminders;
pub mod repo_assess;
pub mod repo_daily;
pub mod repo_determinations;
pub mod repo_plans;
pub mod repo_research;
pub mod review;
pub mod seed;
pub mod settings;
pub mod util;

pub use rusqlite;

use thiserror::Error;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Error)]
pub enum Error {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("{0}")]
    Invalid(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("csv error: {0}")]
    Csv(#[from] csv::Error),
}

impl Error {
    pub fn invalid(msg: impl Into<String>) -> Self {
        Error::Invalid(msg.into())
    }
    pub fn not_found(what: impl Into<String>) -> Self {
        Error::NotFound(what.into())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
