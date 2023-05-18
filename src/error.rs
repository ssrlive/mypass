#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("eframe::Error: {0}")]
    Eframe(#[from] eframe::Error),
}
