//!
//!
//!
//!
//!
//!
//!
//!
mod blob;
mod hash;
mod id;
mod tree;
mod commit;
mod tag;
mod sign;
mod pack;
mod idx;

use std::fmt::Display;
use std::fs::File;
use std::fs::create_dir_all;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use bstr::ByteSlice;
use deflate::write::ZlibEncoder;
use deflate::Compression;
use flate2::read::ZlibDecoder;
use anyhow::{Context, Result};

use crate::git::hash::Hash;
use crate::git::id::ID;

use super::errors::GitError;

/// In the git object store format, between the type and size fields has a space character
/// in Hex means 0x20.
#[allow(unused)]
const SPACE: &[u8] = &[0x20];
/// In the git object store format, between the size and trunk data has a special character
/// in Hex means 0x00.
#[allow(unused)]
const NL: &[u8] = &[0x00];
/// In the git object store format, 0x0a is the line feed character in the commit object.
#[allow(unused)]
const LF: &[u8] = &[0x0A];

/// Git Object Types: Blob, Tree, Commit, Tag
#[allow(unused)]
#[derive(PartialEq, Eq, Hash, Ord, PartialOrd, Debug, Clone, Copy)]
pub enum Type {
    Blob,
    Tree,
    Commit,
    Tag,
}

/// Display trait for Git objects type
impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Type::Blob => write!(f, "blob"),
            Type::Tree => write!(f, "tree"),
            Type::Commit => write!(f, "commit"),
            Type::Tag => write!(f, "tag"),
        }
    }
}

///
impl Type {
    ///
    #[allow(unused)]
    fn to_bytes(self) -> Vec<u8> {
        match self {
            Type::Blob => vec![0x62, 0x6c, 0x6f, 0x62],
            Type::Tree => vec![0x74, 0x72, 0x65, 0x65],
            Type::Commit => vec![0x63, 0x6f, 0x6d, 0x6d, 0x69, 0x74],
            Type::Tag => vec![0x74, 0x61, 0x67],
        }
    }

    ///
    #[allow(unused)]
    fn from_string(s: &str) -> Result<Type, GitError> {
        match s {
            "blob" => Ok(Type::Blob),
            "tree" => Ok(Type::Tree),
            "commit" => Ok(Type::Commit),
            "tag" => Ok(Type::Tag),
            _ => Err(GitError::InvalidObjectType(s.to_string())),
        }
    }
}

/// The metadata of git object.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct Metadata {
    pub t: Type,
    pub h: Hash,
    pub id: ID,
    pub size: usize,
    pub data: Vec<u8>,
}

/// Implement function for Metadata
impl Metadata {
    /// Write the object to the file system with folder and file.
    #[allow(unused)]
    pub(crate) fn write_to_file(&self, root_path: String) -> Result<String, GitError> {
        let mut encoder = ZlibEncoder::new(Vec::new(),
                                           Compression::Default);
        encoder.write_all(&self.data).expect("Write error!");
        let compressed_data =
            encoder.finish().expect("Failed to finish compression!");

        let mut path = PathBuf::from(root_path);
        path.push(&self.id.to_folder());
        create_dir_all(&path).with_context(|| {
            format!("Failed to create directory: {}", path.display())
        }).unwrap();

        path.push(&self.id.to_filename());

        let mut file = File::create(&path).with_context(|| {
            format!("Failed to create file: {}", path.display())
        }).unwrap();
        file.write_all(&compressed_data).with_context(|| {
            format!("Failed to write to file: {}", path.display())
        }).unwrap();

        Ok(path.to_str().unwrap().to_string())
    }

    /// Read the object from the file system and parse to a metadata object.
    #[allow(unused)]
    pub(crate) fn read_object_from_file(path: String) -> Result<Metadata, GitError> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;

        let mut decoder = ZlibDecoder::new(&data[..]);
        let mut decoded = Vec::new();
        decoder.read_to_end(&mut decoded)?;

        let type_index = decoded.find_byte(0x20).unwrap();
        let t = &decoded[0..type_index];

        let size_index = decoded.find_byte(0x00).unwrap();
        let size = decoded[type_index + 1..size_index].iter().copied()
            .map(|x| x as char)
            .collect::<String>()
            .parse::<usize>()
            .unwrap();

        let mut data = decoded[size_index + 1..].to_vec();

        match String::from_utf8(t.to_vec()).unwrap().as_str() {
            "blob" => {
                Ok(Metadata {
                    t: Type::Blob,
                    h: Hash::Sha1,
                    id: ID::from_vec(Type::Blob, &mut data),
                    size,
                    data,
                })
            }
            "tree" => {
                Ok(Metadata {
                    t: Type::Tree,
                    h: Hash::Sha1,
                    id: ID::from_vec(Type::Tree, &mut data),
                    size,
                    data,
                })
            }
            "commit" => {
                Ok(Metadata {
                    t: Type::Commit,
                    h: Hash::Sha1,
                    id: ID::from_vec(Type::Commit, &mut data),
                    size,
                    data,
                })
            }
            "tag" => {
                Ok(Metadata {
                    t: Type::Tag,
                    h: Hash::Sha1,
                    id: ID::from_vec(Type::Tag, &mut data),
                    size,
                    data,
                })
            }
            _ => Err(GitError::InvalidObjectType(String::from_utf8(t.to_vec()).unwrap())),
        }
    }
}

///
#[cfg(test)]
mod tests {

}