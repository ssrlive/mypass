#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("eframe::Error: {0}")]
    Eframe(#[from] eframe::Error),

    #[error("keepass_ng::DatabaseOpenError: {0}")]
    KeepassDatabaseOpenError(#[from] keepass_ng::DatabaseOpenError),

    #[error("keepass_ng::DatabaseKeyError: {0}")]
    KeepassDatabaseKeyError(#[from] keepass_ng::DatabaseKeyError),

    #[error("std::io::Error: {0}")]
    StdIoError(#[from] std::io::Error),

    #[error("dotenvy::Error: {0}")]
    DotenvyError(#[from] dotenvy::Error),

    #[error("std::env::VarError: {0}")]
    StdEnvVarError(#[from] std::env::VarError),

    #[error("keepass_ng::Error: {0}")]
    KeepassError(#[from] keepass_ng::Error),

    #[error("&str error: {0}")]
    Str(String),

    #[error("String error: {0}")]
    String(String),

    #[error("&String error: {0}")]
    RefString(String),
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Str(s.to_string())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::String(s)
    }
}

impl From<&String> for Error {
    fn from(s: &String) -> Self {
        Error::RefString(s.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
