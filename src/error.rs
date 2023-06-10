#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("eframe::Error: {0}")]
    Eframe(#[from] eframe::Error),

    #[error("keepass::error::DatabaseOpenError: {0}")]
    KeepassDatabaseOpenError(#[from] keepass::error::DatabaseOpenError),

    #[error("keepass::error::DatabaseKeyError: {0}")]
    KeepassDatabaseKeyError(#[from] keepass::error::DatabaseKeyError),

    #[error("std::io::Error: {0}")]
    StdIoError(#[from] std::io::Error),

    #[error("dotenvy::Error: {0}")]
    DotenvyError(#[from] dotenvy::Error),

    #[error("std::env::VarError: {0}")]
    StdEnvVarError(#[from] std::env::VarError),
}

pub type Result<T> = std::result::Result<T, Error>;
