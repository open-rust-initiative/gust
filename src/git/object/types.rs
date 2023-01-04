//! ### Types enums for object types
//! There are ObjectType
//! PackObjectType
//!
//!

use std::{fmt::Display, vec};

use crate::git::errors::GitError;

/// Four abstract Object Types:
/// - Blob
/// - Tree
/// - Commit
/// - Tag
/// - OffsetDelta(6)
/// - HashDelta(7)
#[derive(PartialEq, Eq, Hash, Ord, PartialOrd, Debug, Clone, Copy)]
pub enum ObjectType {
    Commit,
    Tree,
    Blob,
    Tag,
    OffsetDelta,
    HashDelta,
}

/// Display trait for Git objects type
impl Display for ObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ObjectType::Blob => write!(f, "blob"),
            ObjectType::Tree => write!(f, "tree"),
            ObjectType::Commit => write!(f, "commit"),
            ObjectType::Tag => write!(f, "tag"),
            ObjectType::OffsetDelta => write!(f, "OffsetDelta"),
            ObjectType::HashDelta => write!(f, "HashDelta"),
        }
    }
}

///
impl ObjectType {
    ///
    #[allow(unused)]
    pub fn to_bytes(self) -> Vec<u8> {
        match self {
            ObjectType::Blob => vec![0x62, 0x6c, 0x6f, 0x62],
            ObjectType::Tree => vec![0x74, 0x72, 0x65, 0x65],
            ObjectType::Commit => vec![0x63, 0x6f, 0x6d, 0x6d, 0x69, 0x74],
            ObjectType::Tag => vec![0x74, 0x61, 0x67],
            _ => vec![],
        }
    }

    ///
    #[allow(unused)]
    pub fn from_string(s: &str) -> Result<ObjectType, GitError> {
        match s {
            "blob" => Ok(ObjectType::Blob),
            "tree" => Ok(ObjectType::Tree),
            "commit" => Ok(ObjectType::Commit),
            "tag" => Ok(ObjectType::Tag),
            _ => Err(GitError::InvalidObjectType(s.to_string())),
        }
    }

    ///
    #[allow(unused)]
    pub fn type2_number(&self) -> u8 {
        match self {
            ObjectType::Commit => 1,
            ObjectType::Tree => 2,
            ObjectType::Blob => 3,
            ObjectType::Tag => 4,
            ObjectType::OffsetDelta => 6,
            ObjectType::HashDelta => 7,
        }
    }

    ///
    #[allow(unused)]
    pub fn number_type(num: u8) -> Self {
        match num {
            1 => ObjectType::Commit,
            2 => ObjectType::Tree,
            3 => ObjectType::Blob,
            4 => ObjectType::Tag,
            6 => ObjectType::OffsetDelta,
            7 => ObjectType::HashDelta,
            _ => panic!("Invalid Git object types"),
        }
    }
}
