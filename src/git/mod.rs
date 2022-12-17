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
mod midx;
mod object;
mod pack;

// use std::fmt::Display;
// use std::fs::{File,create_dir_all};
// use std::io::{BufReader,Read,Write};

// use std::path::PathBuf;
// use anyhow::{Context, Result};
// use bstr::ByteSlice;
// use deflate::write::ZlibEncoder;
// use deflate::Compression;
// use flate2::read::ZlibDecoder;

// use self::hash::{HashType, Hash};
// use self::object::Object;
// use self::object::types::ObjectType;

// use super::errors::GitError;

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
    #[test]
    fn test_a_single_blob() {
        // let metadata = Metadata::
        // blob::Blob::new(metadata);
    }
}
