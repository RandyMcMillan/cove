use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("failed to check node url: {0}")]
    CheckUrlError(#[from] crate::client::Error),
}