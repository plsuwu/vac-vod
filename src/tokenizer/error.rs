use std::io;

use thiserror::Error;

pub type Result<T> = core::result::Result<T, TokenizerError>;

#[derive(Debug, Error)]
pub enum TokenizerError {
    #[error(transparent)]
    IoErr(#[from] io::Error),
}

