//!
//!
//!
//!
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GustError {
    #[error(transparent)]
    IOError(#[from] std::io::Error),
}