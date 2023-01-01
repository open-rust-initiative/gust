//!
//!
//!
//!
//!
//!
//!
//!

pub mod hash;

mod id;
mod idx;
mod object;
mod pack;
mod utils;

pub(crate) mod errors;

/// In the git object store format, between the type and size fields has a space character
/// in Hex means 0x20.
pub const SPACE: &[u8] = &[0x20];

/// In the git object store format, between the size and trunk data has a special character
/// in Hex means 0x00.
pub const NL: &[u8] = &[0x00];

/// In the git object store format, 0x0a is the line feed character in the commit object.
// pub const LF: &[u8] = &[0x0A];

///
#[cfg(test)]
mod tests {

}
