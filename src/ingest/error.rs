use std::io;

use thiserror::Error;

pub type Result<T> = core::result::Result<T, ParserError>;

#[derive(Debug, Error)]
pub enum ParserError {
    #[error(transparent)]
    IoErr(#[from] io::Error),
}
