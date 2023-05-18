#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("eframe::Error: {0}")]
    Eframe(#[from] eframe::Error),

    #[error("keepass::error::DatabaseOpenError: {0}")]
    KeepassDatabaseOpenError(#[from] keepass::error::DatabaseOpenError),

    #[error("std::io::Error: {0}")]
    StdIoError(#[from] std::io::Error),

    #[error("dotenvy::Error: {0}")]
    DotenvyError(#[from] dotenvy::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
