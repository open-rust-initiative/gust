use std::io::{Error, ErrorKind}; 
pub fn make_error(message: &str) -> Error {
    Error::new(ErrorKind::Other, message)
}
  