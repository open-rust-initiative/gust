//!
//!
//!
//!
//!
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::File;
use std::fs::create_dir_all;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::io::Cursor;

use byteorder::{BigEndian, ReadBytesExt};
use bstr::ByteSlice;
use crypto::{digest::Digest, sha1::Sha1};
use deflate::write::ZlibEncoder;
use deflate::Compression;
use flate2::read::ZlibDecoder;
use anyhow::{Context, Result};

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
    //
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

/// Git Object hash type. only support SHA1 for now.
#[allow(unused)]
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
pub enum Hash {
    Sha1,
}

/// Display trait for Hash type
impl Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Hash::Sha1 => write!(f, "sha1"),
        }
    }
}

/// Git Object ID: a SHA-1 hash for now, and we will support multiple hash algorithms later.
/// The SHA-1 Hax ID is a 40-byte hexadecimal string.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct ID {
    pub bytes: Vec<u8>,
    pub hash: String,
}

///
impl ID {
    /// Generate ID base on the git object type, size and data and return the ID, the object and data length.
    #[allow(unused)]
    fn generate(t: Type, data: &mut [u8]) -> Result<(Vec<u8>, Vec<u8>, usize)> {
        let mut hash = Sha1::new();

        let object: &[u8] = &[
            t.to_string().as_bytes(),
            SPACE,
            data.len().to_string().as_bytes(),
            NL,
            (data),
        ].concat();

        hash.input(object);
        let mut id = [0u8; 20];
        hash.result(&mut id);

        Ok((id.to_vec(), object.to_vec(), data.len()))
    }

    /// Return the first and second alphanumeric characters of the ID.
    /// In the git object store format, the first two characters is the folder for save the object.
    #[allow(unused)]
    fn to_folder(&self) -> String {
        self.hash.as_str()[0..2].to_string()
    }

    /// Return the last 18 characters of the ID for the object name.
    #[allow(unused)]
    fn to_filename(&self) -> String {
        self.hash.as_str()[2..].to_string()
    }

    /// Return the ID in the git object store format from a byte array.
    #[allow(unused)]
    fn from_bytes(bytes: &[u8]) -> Self {
        ID {
            bytes: bytes.to_vec(),
            hash: hex::encode(bytes),
        }
    }

    /// Return the ID in the git object store format form a hex string.
    #[allow(unused)]
    fn from_string(s: &str) -> Self {
        ID {
            bytes: hex::decode(s).unwrap(),
            hash: s.to_string(),
        }
    }
}

/// Display ObjectID hash data to hex string
impl Display for ID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", &self.hash)
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
    fn write_to_file(&self, root_path: String) -> Result<String, GitError> {
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

    // fn read_object_from_data(data: &[u8]) -> Result<Metadata, GitError> {
    //     let mut decoder = ZlibDecoder::new(data);
    //     let mut decoded_data = Vec::new();
    //     decoder.read_to_end(&mut decoded_data).expect("Decode error!");
    //
    //     let mut decoded_data_iter = decoded_data.iter();
    //     let mut t_bytes = Vec::new();
    //     let mut size_bytes = Vec::new();
    //     let mut data_bytes = Vec::new();
    //
    //     // read the type
    //     for _ in 0..4 {
    //         t_bytes.push(decoded_data_iter.next().unwrap().clone());
    //     }
    //
    //     // read the size
    //     for _ in 0..10 {
    //         size_bytes.push(decoded_data_iter.next().unwrap().clone());
    //     }
    //
    //     // read the data
    //     for _ in 0..usize::from_str_radix(
    //         str::from_utf8(&size_bytes).unwrap(), 10).unwrap() {
    //         data_bytes.push(decoded_data_iter.next().unwrap().clone());
    //     }
    //
    //     let t = Type::from_string(str::from_utf8(&t_bytes).unwrap()).unwrap();
    //     let id = ID::from_bytes(&data_bytes);
    //
    //     Ok(Metadata {
    //         t: t,
    //         h: Hash::Sha1,
    //         id: id,
    //         size: data_bytes.len(),
    //         data: data_bytes,
    //     })
    // }

    /// Read the object from the file system and parse to a metadata object.
    #[allow(unused)]
    fn read_object_from_file(path: String) -> Result<Metadata, GitError> {
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
                let (bytes, _, _) = ID::generate(Type::Blob, &mut data).with_context(|| {
                    format!("Failed to generate ID for blob data: {:?}", data)
                }).unwrap();
                Ok(Metadata {
                    t: Type::Blob,
                    h: Hash::Sha1,
                    id: ID::from_bytes(&bytes),
                    size,
                    data,
                })
            }
            "tree" => {
                let (bytes, _, _) = ID::generate(Type::Tree, &mut data).with_context(|| {
                    format!("Failed to generate ID for tree data: {:?}", data)
                }).unwrap();
                Ok(Metadata {
                    t: Type::Tree,
                    h: Hash::Sha1,
                    id: ID::from_bytes(&bytes),
                    size,
                    data,
                })
            }
            "commit" => {
                let (bytes, _, _) = ID::generate(Type::Commit, &mut data).with_context(|| {
                    format!("Failed to generate ID for commit data: {:?}", data)
                }).unwrap();
                Ok(Metadata {
                    t: Type::Commit,
                    h: Hash::Sha1,
                    id: ID::from_bytes(&bytes),
                    size,
                    data,
                })
            }
            "tag" => {
                let (bytes, _, _) = ID::generate(Type::Tag, &mut data).with_context(|| {
                    format!("Failed to generate ID for tag data: {:?}", data)
                }).unwrap();
                Ok(Metadata {
                    t: Type::Tag,
                    h: Hash::Sha1,
                    id: ID::from_bytes(&bytes),
                    size,
                    data,
                })
            }
            _ => Err(GitError::InvalidObjectType(String::from_utf8(t.to_vec()).unwrap())),
        }
    }
}

/// Git Object: blob
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct Blob {
    pub meta: Metadata,
    pub data: Vec<u8>,
}

impl Blob {
    ///
    #[allow(unused)]
    fn write_to_file(&self, root_path: String) -> Result<String, GitError> {
        self.meta.write_to_file(root_path)
    }

    ///
    #[allow(unused)]
    fn to_tree_item(&self, filename: String) -> Result<TreeItem, ()> {
        Ok(
            TreeItem {
                mode: TreeItemType::Blob.to_bytes().to_vec(),
                item_type: TreeItemType::Blob,
                id: self.meta.id.clone(),
                filename,
            }
        )

    }
}

///
#[derive(PartialEq, Eq, Hash, Ord, PartialOrd, Debug, Clone, Copy)]
pub enum TreeItemType {
    Blob,
    BlobExecutable,
    Tree,
    Commit,
    Link,
}

///
impl Display for TreeItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            TreeItemType::Blob => write!(f, "blob"),
            TreeItemType::BlobExecutable => write!(f, "blob executable"),
            TreeItemType::Tree => write!(f, "tree"),
            TreeItemType::Commit => write!(f, "commit"),
            TreeItemType::Link => write!(f, "link"),
        }
    }
}

impl TreeItemType {
    ///
    #[allow(unused)]
    fn to_bytes(self) -> &'static [u8] {
        match self {
            TreeItemType::Blob => b"100644",
            TreeItemType::BlobExecutable => b"100755",
            TreeItemType::Tree => b"40000",
            TreeItemType::Link => b"120000",
            TreeItemType::Commit => b"160000",
        }
    }

    ///
    #[allow(unused)]
    fn tree_item_type_from(mode: &[u8]) -> Result<TreeItemType, GitError> {
        Ok(match mode {
            b"40000" => TreeItemType::Tree,
            b"100644" => TreeItemType::Blob,
            b"100755" => TreeItemType::BlobExecutable,
            b"120000" => TreeItemType::Link,
            b"160000" => TreeItemType::Commit,
            b"100664" => TreeItemType::Blob,
            b"100640" => TreeItemType::Blob,
            _ => return Err(GitError::InvalidTreeItem(String::from_utf8(mode.to_vec()).unwrap())),
        })
    }
}

/// Git Object: tree item
pub struct TreeItem {
    pub mode: Vec<u8>,
    pub item_type: TreeItemType,
    pub id: ID,
    pub filename: String,
}

/// Git Object: tree
pub struct Tree {
    pub meta: Metadata,
    pub tree_items: Vec<TreeItem>,
}

impl Display for Tree {
    #[allow(unused)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for item in &self.tree_items {
            writeln!(f, "{} {} {} {}",
                   String::from_utf8(item.mode.to_vec()).unwrap(),
                   item.item_type, item.id, item.filename);
        }

        Ok(())
    }
}

impl Tree {
    ///
    #[allow(unused)]
    fn decode_metadata(&mut self) -> Result<(), GitError> {
        let mut tree_items:Vec<TreeItem> = Vec::new();
        let mut index = 0;

        while index < self.meta.data.len() {
            let mode_index = &self.meta.data[index..].find_byte(0x20).unwrap();
            let mode = &self.meta.data[index..index + *mode_index];
            let item_type = TreeItemType::tree_item_type_from(mode).unwrap();

            let filename_index = &self.meta.data[index..].find_byte(0x00).unwrap();
            let filename = String::from_utf8(self.meta.data[index + mode_index + 1.. index + *filename_index]
                .to_vec())
                .unwrap();

            let id = ID::from_bytes(&self.meta.data[index + filename_index + 1..index + filename_index + 21]);

            self.tree_items.push(TreeItem {
                mode: mode.to_vec(),
                item_type,
                id,
                filename,
            });

            index = index + filename_index + 21;
        }

        Ok(())
    }

    ///
    #[allow(unused)]
    fn encode_metadata(&self) -> Result<Metadata, ()> {
        let mut data = Vec::new();
        for item in &self.tree_items {
            data.extend_from_slice(&item.mode);
            data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
            data.extend_from_slice(item.filename.as_bytes());
            data.extend_from_slice(0x00u8.to_be_bytes().as_ref());
            data.extend_from_slice(&item.id.bytes);
        }

        let (bytes, _, _) = ID::generate(Type::Tree, &mut data).with_context(|| {
            format!("Failed to generate ID for tree data: {:?}", data)
        }).unwrap();

        Ok(
            Metadata {
                t: Type::Tree,
                h: Hash::Sha1,
                id: ID::from_bytes(bytes.as_slice()),
                size: data.len(),
                data,
            },
        )
    }

    ///
    #[allow(unused)]
    fn write_to_file(&self, root_path: String) -> Result<String, GitError> {
        self.meta.write_to_file(root_path)
    }
}

///
#[allow(unused)]
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct AuthorSign {
    pub t: String,
    pub name: String,
    pub email: String,
    pub timestamp: usize,
    pub timezone: String,
}

impl Display for AuthorSign {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} <{}> {} {}", self.name, self.email, self.timestamp, self.timezone)
    }
}

impl AuthorSign {
    ///
    #[allow(unused)]
    fn decode_from_data(&mut self, data: Vec<u8>) -> Result<(), GitError> {
        let mut data = data;

        let name_start = data.find_byte(0x20).unwrap();

        self.t = String::from_utf8(data[..name_start].to_vec()).unwrap();

        let email_start = data.find_byte(0x3C).unwrap();
        let email_end = data.find_byte(0x3E).unwrap();

        self.name = data[name_start + 1..email_start - 1].to_str().unwrap().to_string();
        self.email = data[email_start + 1..email_end].to_str().unwrap().to_string();
        data = data[email_end + 2..].to_vec();

        let timestamp_split = data.find_byte(0x20).unwrap();
        self.timestamp = data[0..timestamp_split].to_str().unwrap().parse::<usize>().unwrap();
        self.timezone = data[timestamp_split + 1..].to_str().unwrap().to_string();

        Ok(())
    }

    ///
    #[allow(unused)]
    fn encode_to_data(&self) -> Result<Vec<u8>, GitError> {
        let mut data = Vec::new();

        data.extend_from_slice(self.t.as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.name.as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(0x3Cu8.to_be_bytes().as_ref());
        data.extend_from_slice(self.email.as_bytes());
        data.extend_from_slice(0x3Eu8.to_be_bytes().as_ref());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.timestamp.to_string().as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.timezone.as_bytes());

        Ok(data)
    }
}

/// Git Object: commit
#[allow(unused)]
pub struct Commit {
    pub meta: Metadata,
    pub tree_id: ID,
    pub parent_tree_ids: Vec<ID>,
    pub author: AuthorSign,
    pub committer: AuthorSign,
    pub message: String,
}

impl Commit {
    ///
    #[allow(unused)]
    fn decode_meta(&mut self) -> Result<(), GitError> {
        let mut data = self.meta.data.clone();

        // Find the tree id and remove it from the data
        let tree_begin = data.find_byte(0x20).unwrap();
        let tree_end = data.find_byte(0x0a).unwrap();
        self.tree_id = ID::from_string(data[tree_begin + 1..tree_end].to_str().unwrap());
        data = data[tree_end + 1..].to_vec();

        // Find the parent tree ids and remove them from the data
        let author_begin = data.find("author").unwrap();
        if data.find_iter("parent").count() > 0 {
            let mut parents:Vec<ID> = Vec::new();
            let mut index = 0;

            while index < author_begin {
                let parent_begin = data.find_byte(0x20).unwrap();
                let parent_end = data.find_byte(0x0a).unwrap();
                parents.push(ID::from_string(data[parent_begin + 1..parent_end].to_str().unwrap()));
                index = index + parent_end + 1;
            }

            self.parent_tree_ids = parents;
        }
        data = data[author_begin..].to_vec();

        // Find the author and remove it from the data
        let author_data = data[.. data.find_byte(0x0a).unwrap()].to_vec();
        self.author.decode_from_data(author_data)?;
        data = data[data.find_byte(0x0a).unwrap() + 1..].to_vec();

        // Find the committer and remove it from the data
        let committer_data = data[..data.find_byte(0x0a).unwrap()].to_vec();
        self.committer.decode_from_data(committer_data)?;
        self.message = data[data.find_byte(0x0a).unwrap() + 1..].to_vec().to_str().unwrap().to_string();

        Ok(())
    }

    ///
    #[allow(unused)]
    fn write_to_file(&self, root_path: String) -> Result<String, GitError> {
        self.meta.write_to_file(root_path)
    }

    ///
    #[allow(unused)]
    fn encode_metadata(&self) -> Result<Metadata, ()> {
        let mut data = Vec::new();

        data.extend_from_slice("tree".as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.tree_id.to_string().as_bytes());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());

        for parent_tree_id in &self.parent_tree_ids {
            data.extend_from_slice("parent".as_bytes());
            data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
            data.extend_from_slice(parent_tree_id.to_string().as_bytes());
            data.extend_from_slice(0x0au8.to_be_bytes().as_ref());
        }

        data.extend_from_slice(self.author.encode_to_data().unwrap().as_ref());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());
        data.extend_from_slice(self.committer.encode_to_data().unwrap().as_ref());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());
        data.extend_from_slice(self.message.as_bytes());

        let (bytes, _, _) = ID::generate(Type::Commit, &mut data).with_context(|| {
            format!("Failed to generate ID for tree data: {:?}", data)
        }).unwrap();

        Ok(
            Metadata {
                t: Type::Commit,
                h: Hash::Sha1,
                id: ID::from_bytes(bytes.as_slice()),
                size: data.len(),
                data,
        })
    }
}

///
impl Display for Commit {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "tree {}\n", self.tree_id)?;

        for parent in self.parent_tree_ids.iter() {
            writeln!(f, "parent {}\n", parent)?;
        }

        writeln!(f, "author {}\n", self.author)?;
        writeln!(f, "committer {}\n", self.committer)?;
        writeln!(f, "\n{}", self.message)
    }
}

/// Git Object: tag
#[allow(unused)]
pub struct Tag {
    pub meta: Metadata,
    pub object: ID,
    pub t: Type,
    pub tag: String,
    pub tagger: AuthorSign,
    pub message: String,
}

impl Tag {
    ///
    #[allow(unused)]
    fn decode_metadata(&mut self) -> Result<(), GitError> {
        let mut data = self.meta.data.clone();

        let object_begin = data.find_byte(0x20).unwrap();
        let object_end = data.find_byte(0x0a).unwrap();
        self.object = ID::from_string(data[object_begin + 1..object_end].to_str().unwrap());
        data = data[object_end + 1..].to_vec();

        let type_begin = data.find_byte(0x20).unwrap();
        let type_end = data.find_byte(0x0a).unwrap();
        self.t = Type::from_string(data[type_begin + 1..type_end].to_str().unwrap()).unwrap();
        data = data[type_end + 1..].to_vec();

        let tag_begin = data.find_byte(0x20).unwrap();
        let tag_end = data.find_byte(0x0a).unwrap();
        self.tag = data[tag_begin + 1..tag_end].to_str().unwrap().parse().unwrap();
        data = data[type_end..].to_vec();

        let tagger = data.find("tagger").unwrap();
        let tagger_data = data[.. data.find_byte(0x0a).unwrap()].to_vec();
        self.tagger.decode_from_data(tagger_data)?;
        data = data[data.find_byte(0x0a).unwrap() + 1..].to_vec();

        self.message = data[data.find_byte(0x0a).unwrap()..].to_vec().to_str().unwrap().to_string();

        Ok(())
    }

    ///
    #[allow(unused)]
    fn encode_metadata(&self) -> Result<Metadata, ()> {
        let mut data = Vec::new();

        data.extend_from_slice("object".as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.object.to_string().as_bytes());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());

        data.extend_from_slice("type".as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.t.to_string().as_bytes());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());

        data.extend_from_slice("tag".as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.tag.as_bytes());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());

        data.extend_from_slice(self.tagger.encode_to_data().unwrap().as_ref());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());
        data.extend_from_slice(self.message.as_bytes());

        let (bytes, _, _) = ID::generate(Type::Tag, &mut data).with_context(|| {
            format!("Failed to generate ID for tree data: {:?}", data)
        }).unwrap();

        Ok(
            Metadata {
                t: Type::Tag,
                h: Hash::Sha1,
                id: ID::from_bytes(bytes.as_slice()),
                size: data.len(),
                data,
            })
    }

    ///
    #[allow(unused)]
    fn write_to_file(&self, root_path: String) -> Result<String, GitError> {
        self.meta.write_to_file(root_path)
    }
}

///
#[allow(unused)]
struct Pack {
    head: String,
    version: u32,
    number_of_objects: u32,
    signature: ID,
}

impl Pack {
    /// Git [Pack Format](https://github.com/git/git/blob/master/Documentation/technical/pack-format.txt)
    #[allow(unused)]
    fn decode(&mut self, mut data: Vec<u8>) -> Result<(), GitError> {
        let mut index = 0;

        // 4-byte signature:
        //          The signature is: {'P', 'A', 'C', 'K'}
        if data[0..4].to_vec() != vec![80, 65, 67, 75] {
            return Err(GitError::InvalidPackFile(format!("Invalid pack header: {:?}", data[0..4].to_vec())));
        }
        self.head = data[0..4].to_vec().to_str().unwrap().to_string();
        index += 4;

        //4-byte version number (network byte order):
        // 	 Git currently accepts version number 2 or 3 but generates version 2 only.
        //[0,0,0,2] for version 2, [0,0,0,3] for version 3.
        let mut v = Cursor::new(data[index..8].to_vec());
        self.version = v.read_u32::<BigEndian>().unwrap();
        index += 4;

        //4-byte number of objects contained in the pack (network byte order)
        // Observation: we cannot have more than 4G versions ;-) and more than 4G objects in a pack.
        // So we can safely ignore the 4-byte number of objects.
        let mut n = Cursor::new(data[index..12].to_vec());
        self.number_of_objects = n.read_u32::<BigEndian>().unwrap();
        index += 4;

        self.signature = ID::from_bytes(&data[data.len() - 20..data.len()]);

        Ok(())
    }

    #[allow(unused)]
    fn next_object(&self, data: &mut [u8], index: &mut usize) -> Result<usize, GitError> {
        let mut offset = *index;

        let mut byte = data[offset];
        offset += 1;


        let object_type = (byte & 0x70) >> 4;
        let mut _object_size = (byte & 0xf) as u64;

        let mut consumed = 0;
        let mut continuation = byte & 0x80;
        loop {
            if continuation < 1 {
                break;
            }

            byte = data[offset];
            offset += 1;
            continuation = byte & 0x80;

            _object_size |= ((byte & 0x7f) as u64) << (4 + 7 * consumed);
            consumed += 1;
        }

        match object_type {
            0..=4 => {
                // 1：commit; 2: tree; 3: blob; 4: tag
                let mut deflate_stream = ZlibDecoder::new(&data[offset..]);
                let mut data = Vec::new();
                deflate_stream.read_to_end(&mut data)?;
                offset += deflate_stream.total_in() as usize;

                Ok(offset)
            },
            6 => {
                // OFS_DELTA 对象解析逻辑
                byte = data[offset];
                offset += 1;
                let mut _negative_offset = u64::from(byte & 0x7F);

                while byte & 0x80 > 0 {
                    _negative_offset += 1;
                    _negative_offset <<= 7;
                    byte = data[offset];
                    offset += 1;
                    _negative_offset += u64::from(byte & 0x7F);
                }

                let mut deflate_stream = ZlibDecoder::new(&data[offset..]);
                let mut instructions = Vec::new();
                deflate_stream.read_to_end(&mut instructions)?;
                offset += deflate_stream.total_in() as usize;

                Ok(offset)
            },
            7 => {
                // REF_DELTA 偏移
                offset += 20;

                let mut deflate_stream = ZlibDecoder::new(&data[offset..]);
                let mut instructions = Vec::new();
                deflate_stream.read_to_end(&mut instructions)?;
                offset += deflate_stream.total_in() as usize;

                Ok(offset)
            },
            _ => Err(GitError::InvalidObjectType(object_type.to_string())),
        }
    }
}

///
#[allow(unused)]
struct IdxItem {
    id: ID,
    crc32: String,
    offset: usize,
}

impl Display for IdxItem {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} {} ({})", self.offset, self.id, self.crc32)
    }
}

///
#[allow(unused)]
struct Idx {
    version: u32,
    number_of_objects: usize,
    map_of_prefix: HashMap<String, usize>,
    idx_items: Vec<IdxItem>,
    pack_signature: ID,
    idx_signature: ID,
}

impl Idx {
    ///
    #[allow(unused)]
    fn sha1_prefix(&self, n: usize) -> String {
        let pre = format!("{:x}", n);

        if pre.len() == 1 {
            format!("0{}", pre)
        } else {
            pre
        }
    }

    ///
    #[allow(unused)]
    fn decode(& mut self, data: Vec<u8>) -> Result<(), GitError> {
        let mut offset : usize = 0;

        let mut id_of_objects: Vec<ID> = Vec::new();
        let mut crc32_of_objects: Vec<String> = Vec::new();

        // 4-byte Header:
        if data[offset..4].to_vec() != vec![255, 116, 79, 99] {
            return Err(GitError::InvalidIdxFile(format!("Invalid idx header: {:?}", data[0..4].to_vec())));
        }
        offset += 4;

        // 4-byte version number (network byte order):
        let mut v = Cursor::new(data[offset..8].to_vec());
        self.version = v.read_u32::<BigEndian>().unwrap();
        offset += 4;

        // Layer 1:
        //  Number of objects in the pack (network byte order)
        //  The prefix of the SHA-1 hash of the object has how many objects it is in the pack.
        let mut n : usize = 0;
        for i in (offset..offset + 256 * 4).filter(|x| ((x - offset) % 4 == 0)) {
            let mut v = Cursor::new(data[i..i + 4].to_vec());
            let m = v.read_u32::<BigEndian>().unwrap() as usize;

            if m != n {
                self.map_of_prefix.insert(self.sha1_prefix((i - 8)/4), m - n);
                self.number_of_objects = m;
                n = m;
            }
        }
        offset += 256 * 4; // 1040

        // Layer 2:
        //  The all the SHA-1 hashes of the objects in the pack.
        for i in (offset..offset + (20 * n) as usize).filter(|x| ((x - offset) % 20 == 0))  {
            let id = ID::from_bytes(&data[(i as usize)..(i as usize) + 20].to_vec());
            id_of_objects.push(id);
        }
        offset += 20 * n as usize;


        // Layer 3:
        //   The CRC32 of the object data.
        for i in (offset..offset + (4 * n) as usize).filter(|x| ((x - offset) % 4 == 0)) {
            crc32_of_objects.push(hex::encode(&data[i..i + 4]));
        }
        offset += 4 * n as usize;


        // Layer 4:
        //   the object offset in the pack file.
        let mut index = 0;
        for (index, i) in (offset..offset + (4 * n) as usize).filter(|x| ((x - offset) % 4 == 0)).enumerate() {
            let mut v = Cursor::new(data[i..i + 4].to_vec());
            let m = v.read_u32::<BigEndian>().unwrap() as usize;

            self.idx_items.push(IdxItem {
                id: id_of_objects[index].clone(),
                crc32: crc32_of_objects[index].clone(),
                offset: m,
            });
        }
        offset += 4 * n as usize;

        // Layer 5

        // Layer 6:
        //  The SHA-1 hash of the pack file itself.
        //  The SHA-1 hash of the index file itself.
        self.pack_signature = ID::from_bytes(&data[offset..offset + 20]);
        offset += 20;
        self.idx_signature = ID::from_bytes(&data[offset..]);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::BufReader;
    use std::io::Read;
    use std::path::Path;
    use std::path::PathBuf;

    use anyhow::Context;
    use bstr::ByteSlice;

    /// There is a bug need to be resolve:
    ///     The `\r\n` is a Windows Style, but the `\n` is a POSIX Style.
    ///     The file will be different both length and content between Windows and Mac.
    ///     So there is different SHA-1 value.
    ///
    ///     Temporarily, just replace the `\r\n` to `\n` in the test.
    ///
    ///     Same as the another test case: [test_blob_write_to_file]
    ///
    ///     References:
    ///         [1] https://docs.github.com/cn/get-started/getting-started-with-git/configuring-git-to-handle-line-endings
    ///
    #[test]
    fn test_object_id_new() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/gitmega.md");
        let f = File::open(path).ok();
        let mut reader = BufReader::new(f.unwrap());
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).ok();

        if env::consts::OS == "windows" {
            buffer = buffer.replace(b"\r\n", b"\n");
        }

        let (bytes, _, _) = super::ID::generate(super::Type::Blob, &mut buffer).with_context(|| {
            format!("Failed to generate ID for blob data: {:?}", buffer)
        }).unwrap();

        let id = super::ID::from_bytes(&bytes);
        assert_eq!("82352c3a6a7a8bd32011751699c7a3648d1b5d3c", id.to_string());
    }

    #[test]
    fn test_blob_write_to_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/gitmega.md");
        let f = File::open(path).ok();
        let mut reader = BufReader::new(f.unwrap());
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).ok();

        if env::consts::OS == "windows" {
            buffer = buffer.replace(b"\r\n", b"\n");
        }

        let (bytes, data, size) =
            super::ID::generate(super::Type::Blob, &mut buffer).with_context(|| {
                format!("Failed to generate ID for blob data: {:?}", buffer)
            }).unwrap();

        let meta = super::Metadata {
            t: super::Type::Blob,
            h: super::Hash::Sha1,
            id: super::ID::from_bytes(&bytes),
            size,
            data,
        };

        meta.write_to_file("/tmp".to_string())
            .expect("Write error!");
        assert!(Path::new("/tmp/82/352c3a6a7a8bd32011751699c7a3648d1b5d3c").exists());
    }

    #[test]
    fn test_tree_write_to_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/blob-82352c3a6a7a8bd32011751699c7a3648d1b5d3c-gitmega.md");

        let meta =
            super::Metadata::read_object_from_file(path.to_str().unwrap().to_string())
                .expect("Read error!");

        assert_eq!(meta.t, super::Type::Blob);
        assert_eq!("82352c3a6a7a8bd32011751699c7a3648d1b5d3c", meta.id.to_string());
        assert_eq!(16, meta.size);

        let blob = super::Blob {
            meta: meta.clone(),
            data: meta.data,
        };

        assert_eq!(
            "# Hello Gitmega\n",
            String::from_utf8(blob.clone().data).unwrap().as_str()
        );

        let item = blob
            .to_tree_item(String::from("gitmega.md")).unwrap();

        let mut tree = super::Tree {
            meta: super::Metadata {
                t: super::Type::Tree,
                h: super::Hash::Sha1,
                id: super::ID {
                    bytes: vec![],
                    hash: String::new(),
                },
                size: 0,
                data: vec![]
            },
            tree_items: vec![item],
        };

        tree.meta = tree.encode_metadata().unwrap();
        tree.write_to_file("/tmp".to_string()).expect("Write error!");

        assert!(Path::new("/tmp/1b/dbc1e723aa199e83e33ecf1bb19f874a56ebc3").exists());
    }

    #[test]
    fn test_tree_write_to_file_2_blob() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/blob-fc1a505ac94f98cc5f29100a2d9aef97027a32fb-gitmega.md");

        let meta_gitmega =
            super::Metadata::read_object_from_file(path.to_str().unwrap().to_string())
                .expect("Read error!");

        let blob_gitmega = super::Blob {
            meta: meta_gitmega.clone(),
            data: meta_gitmega.data,
        };

        let item_gitmega = blob_gitmega
            .to_tree_item(String::from("gitmega.md")).unwrap();

        path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/blob-a3b55a2ce16d2429dae2d690d2c15bcf26fbe33c-gust.md");

        let meta_gust =
            super::Metadata::read_object_from_file(path.to_str().unwrap().to_string())
                .expect("Read error!");

        let blob_gust = super::Blob {
            meta: meta_gust.clone(),
            data: meta_gust.data,
        };

        let item_gust = blob_gust
            .to_tree_item(String::from("gust.md")).unwrap();


        let mut tree = super::Tree {
            meta: super::Metadata {
                t: super::Type::Tree,
                h: super::Hash::Sha1,
                id: super::ID {
                    bytes: vec![],
                    hash: String::new(),
                },
                size: 0,
                data: vec![]
            },
            tree_items: vec![item_gitmega, item_gust],
        };

        tree.meta = tree.encode_metadata().unwrap();
        tree.write_to_file("/tmp".to_string()).expect("Write error!");

        assert!(Path::new("/tmp/9b/be4087bedef91e50dc0c1a930c1d3e86fd5f20").exists());
    }

    #[test]
    fn test_blob_read_from_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/blob-82352c3a6a7a8bd32011751699c7a3648d1b5d3c-gitmega.md");

        let meta =
            super::Metadata::read_object_from_file(path.to_str().unwrap().to_string())
                .expect("Read error!");

        assert_eq!(meta.t, super::Type::Blob);

        let blob = super::Blob {
            meta: meta.clone(),
            data: meta.data,
        };

        assert_eq!(
            "82352c3a6a7a8bd32011751699c7a3648d1b5d3c",
            blob.meta.id.to_string()
        );
        assert_eq!(16, blob.meta.size);
        assert_eq!(
            "# Hello Gitmega\n",
            String::from_utf8(blob.data).unwrap().as_str()
        );
    }

    #[test]
    fn test_tree_read_from_file() {
        // 100644 blob 82352c3a6a7a8bd32011751699c7a3648d1b5d3c	gitmega.md
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/tree-1bdbc1e723aa199e83e33ecf1bb19f874a56ebc3");

        let meta = super::Metadata::read_object_from_file(path.to_str().unwrap().to_string())
            .expect("Read error!");

        assert_eq!(super::Type::Tree, meta.t);
        assert_eq!(38, meta.size);

        let mut tree = super::Tree {
            meta,
            tree_items: Vec::new(),
        };

        tree.decode_metadata().unwrap();

        assert_eq!(1, tree.tree_items.len());
        assert_eq!(
            "gitmega.md",
            tree.tree_items[0].filename.as_str()
        );
        assert_eq!(
            "82352c3a6a7a8bd32011751699c7a3648d1b5d3c",
            tree.tree_items[0].id.to_string()
        );
        assert_eq!(
            "100644",
            String::from_utf8(tree.tree_items[0].mode.to_vec()).unwrap().as_str()
        );
        assert_eq!(super::TreeItemType::Blob, tree.tree_items[0].item_type);
    }

    #[test]
    fn test_tree_read_from_file_2_items() {
        // 100644 blob fc1a505ac94f98cc5f29100a2d9aef97027a32fb	gitmega.md
        // 100644 blob a3b55a2ce16d2429dae2d690d2c15bcf26fbe33c	gust.md
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/tree-9bbe4087bedef91e50dc0c1a930c1d3e86fd5f20");

        let meta = super::Metadata::read_object_from_file(path.to_str().unwrap().to_string())
            .expect("Read error!");

        assert_eq!(super::Type::Tree, meta.t);
        assert_eq!(73, meta.size);

        let mut tree = super::Tree {
            meta,
            tree_items: Vec::new(),
        };

        tree.decode_metadata().unwrap();

        assert_eq!(2, tree.tree_items.len());

        assert_eq!(
            "gitmega.md",
            tree.tree_items[0].filename.as_str()
        );
        assert_eq!(
            "fc1a505ac94f98cc5f29100a2d9aef97027a32fb",
            tree.tree_items[0].id.to_string()
        );
        assert_eq!(
            "100644",
            String::from_utf8(tree.tree_items[0].mode.to_vec()).unwrap().as_str()
        );
        assert_eq!(super::TreeItemType::Blob, tree.tree_items[0].item_type);

        assert_eq!(
            "gust.md",
            tree.tree_items[1].filename.as_str()
        );
        assert_eq!(
            "a3b55a2ce16d2429dae2d690d2c15bcf26fbe33c",
            tree.tree_items[1].id.to_string()
        );
        assert_eq!(
            "100644",
            String::from_utf8(tree.tree_items[1].mode.to_vec()).unwrap().as_str()
        );
        assert_eq!(super::TreeItemType::Blob, tree.tree_items[1].item_type);
    }

    #[test]
    fn test_commit_read_from_file_without_parent() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/commit-1b490ec04712d147bbe7c8b3a6d86ed4d3587a6a");

        let meta = super::Metadata::read_object_from_file(path.to_str().unwrap().to_string())
            .expect("Read error!");

        assert_eq!(super::Type::Commit, meta.t);
        assert_eq!(1065, meta.size);
        assert_eq!(
            "1b490ec04712d147bbe7c8b3a6d86ed4d3587a6a",
            meta.id.to_string()
        );

        let mut commit = super::Commit {
            meta,
            tree_id: super::ID { bytes: vec![], hash: "".to_string() },
            parent_tree_ids: vec![],
            author: super::AuthorSign {
                t: "".to_string(),
                name: "".to_string(),
                email: "".to_string(),
                timestamp: 0,
                timezone: "".to_string()
            },
            committer: super::AuthorSign {
                t: "".to_string(),
                name: "".to_string(),
                email: "".to_string(),
                timestamp: 0,
                timezone: "".to_string()
            },
            message: "".to_string()
        };

        commit.decode_meta().unwrap();

        assert_eq!("1bdbc1e723aa199e83e33ecf1bb19f874a56ebc3", commit.tree_id.hash);

    }

    #[test]
    fn test_commit_read_from_file_with_parent() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/commit-3b8bc1e152af7ed6b69f2acfa8be709d1733e1bb");

        let meta = super::Metadata::read_object_from_file(path.to_str().unwrap().to_string())
            .expect("Read error!");

        assert_eq!(super::Type::Commit, meta.t);
        assert_eq!(1126, meta.size);
        assert_eq!(
            "3b8bc1e152af7ed6b69f2acfa8be709d1733e1bb",
            meta.id.to_string()
        );

        let mut commit = super::Commit {
            meta,
            tree_id: super::ID { bytes: vec![], hash: "".to_string() },
            parent_tree_ids: vec![],
            author: super::AuthorSign {
                t: "".to_string(),
                name: "".to_string(),
                email: "".to_string(),
                timestamp: 0,
                timezone: "".to_string()
            },
            committer: super::AuthorSign {
                t: "".to_string(),
                name: "".to_string(),
                email: "".to_string(),
                timestamp: 0,
                timezone: "".to_string()
            },
            message: "".to_string()
        };

        commit.decode_meta().unwrap();

        assert_eq!("9bbe4087bedef91e50dc0c1a930c1d3e86fd5f20", commit.tree_id.to_string());
    }

    #[test]
    fn test_commit_write_to_file() {
        let meta = super::Metadata {
            t: super::Type::Commit,
            h: super::Hash::Sha1,
            size: 0,
            id: super::ID { bytes: vec![], hash: "".to_string() },
            data: vec![],
        };

        let author = super::AuthorSign {
            t: "author".to_string(),
            name: "Quanyi Ma".to_string(),
            email: "eli@patch.sh".to_string(),
            timestamp: 1649521615,
            timezone: "+0800".to_string()
        };

        let committer = super::AuthorSign {
            t: "committer".to_string(),
            name: "Quanyi Ma".to_string(),
            email: "eli@patch.sh".to_string(),
            timestamp: 1649521615,
            timezone: "+0800".to_string()
        };

        let mut commit = super::Commit {
            meta,
            tree_id: super::ID::from_string("9bbe4087bedef91e50dc0c1a930c1d3e86fd5f20"),
            parent_tree_ids: vec![
                super::ID::from_string("1b490ec04712d147bbe7c8b3a6d86ed4d3587a6a"),
            ],
            author,
            committer,
            message:"gpgsig -----BEGIN PGP SIGNATURE-----\n \n iQIzBAABCAAdFiEEanuf5/5ADLU2lvsCZL9E4tsHuXIFAmJRs88ACgkQZL9E4tsH\n uXJAmBAAtubFjLjNzIgal1/Gwy/zlpw7aQvVO2xcX3Xhbeb0UJyKvrSm/Ht19kiz\n 6Bc8ZV75mpKKip93XAljUgWgAO6Q4DUFnVA5bwF1vvhKHbgXLr+I8q+5GqmLW61U\n oBrB/3aJJ/uAxElQz5nOhgB7ztCfeKQ5egbhBXn9QGqPg/RkfQmDPYsU7evk1J0Z\n CyKinbSNe0c92qE95nURzozFb1zf0rO9NtnpYohFCEO5qyuoV4nz7npnJD4Miqy9\n IUQapeJeZC7eDvU8AWbxARrkXQkyfLSebDVcqbz7WfQz+4dhoK7jADaB48oKpR/K\n bKZDJU9a2t2nPC1ojzjQJgXZ6x4linQofBR8wE1ns3W5RoRgcBSj8dQMNH8wXa/T\n oQD6hlCJpjvbiYHuc3tSgCESI4ZU7zGpL9BAQK+C91T8CUamycF1H7TAHXdzNClR\n bWO4EeRzvwZZyIL029DYFxD2IFN7OQb5jc7JvcroIW8jUN0sMPS6jY+d0bg5pgIs\n yJjmI6qPYy7R35OElfTlw8aVSOAnVbQh7MZt6n3JUyezwK9MwbiKdAYKOLYaVaC0\n ++SY+NV4Dwe6W72KhFhxwOJQRGMfES1mRxy4n85BgqfCGy7STGSBOmon3VZEl89z\n rmvdX0JXy93hGH0oUQINsN9bzpsdaQUWVND8wAnb0+sU4LvJz90=\n =9qni\n -----END PGP SIGNATURE-----\n\nAdd gust.md and modify gitmega.md\n\nSigned-off-by: Quanyi Ma <eli@patch.sh>\n".to_string(),
        };

        commit.meta = commit.encode_metadata().unwrap();

        assert_eq!("3b8bc1e152af7ed6b69f2acfa8be709d1733e1bb", commit.meta.id.to_string());

        commit.write_to_file("/tmp".to_string()).expect("Write error!");

        assert!(Path::new("/tmp/3b/8bc1e152af7ed6b69f2acfa8be709d1733e1bb").exists());
    }

    #[test]
    fn test_tag_read_from_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/tag-e5c324b03b72b26f11557c4955c6d17c68dc8595");

        let meta = super::Metadata::read_object_from_file(path.to_str().unwrap().to_string())
            .expect("Read error!");

        assert_eq!(super::Type::Tag, meta.t);
        assert_eq!(976, meta.size);
        assert_eq!("e5c324b03b72b26f11557c4955c6d17c68dc8595", meta.id.to_string());

        let mut tag = super::Tag {
            meta,
            object: super::ID { bytes: vec![], hash: "".to_string() },
            t: super::Type::Commit,
            tag: "".to_string(),
            tagger: super::AuthorSign {
                t: "".to_string(),
                name: "".to_string(),
                email: "".to_string(),
                timestamp: 0,
                timezone: "+0000".to_string()
            },
            message: "".to_string(),
        };

        tag.decode_metadata().unwrap();

        assert_eq!("6414e45babf0bdd043ba40d31123053cfebef26c", tag.object.to_string());
        assert_eq!("commit", tag.t.to_string());
        assert_eq!("v1.1.0", tag.tag);
        assert_eq!(1653037847, tag.tagger.timestamp);
    }

    #[test]
    fn test_tag_write_to_file() {
        let meta = super::Metadata {
            t: super::Type::Tag,
            h: super::Hash::Sha1,
            size: 0,
            id: super::ID { bytes: vec![], hash: "".to_string() },
            data: vec![],
        };

        let tagger = super::AuthorSign {
            t: "tagger".to_string(),
            name: "Quanyi Ma".to_string(),
            email: "eli@patch.sh".to_string(),
            timestamp: 1653037847,
            timezone: "+0800".to_string()
        };

        let mut tag = super::Tag {
            meta,
            object: super::ID::from_string("6414e45babf0bdd043ba40d31123053cfebef26c"),
            t: super::Type::Commit,
            tag: "v1.1.0".to_string(),
            tagger,
            message: "\nIt's a lastest object\n-----BEGIN PGP SIGNATURE-----\n\niQIzBAABCAAdFiEEanuf5/5ADLU2lvsCZL9E4tsHuXIFAmKHWxcACgkQZL9E4tsH\nuXIeFhAAtX+foSvc7/1lb98+QfRjHcpO+LX+LroTaq/QGOTX/2gE+tHD2TJAga1I\nVqDEz8fh8AE366FC7UCjCb5nvsCCox2htzbIxAjsc9L/JckWtxl6WOa/5OZssrDQ\nFtX39BNNl+4TfNn/z1XV+28c9yB1N5HSoP2gzdLoASw3y9n6E0FyzLdoXPILgmJI\nL4DAG/OFkixK+I+TsK+6995497h9BCi3x30dOjfxZS9ptiKhqWulbkflvvM9Cnie\n7obXYmnoe0jBjSfO5GgJlOYcLzE9MMYYzIx47/4lcrCbQXnojkW3KV03PEXGfRCL\nw/y8oBHVvNVRF0Jn+o7F+mzIrbF6Ufku63MfRf7WmbbS3B63CILEjNyuOFoe8mDb\nrmAUffzQSrgnvBk+g01slb6Q+q7Urw6wqHtBPn3ums/inHE9ymTqS7ffmRifUfR8\nD8LvhwpSUI7BdiN6HznRFPxMXzohYIqAJbUltjr4Q7qw/kJI+305Xcs1U5AUIaOp\n77p2UFHRVoMM5mpPOCSwsVJ6cSuOjWXf9afcNMrhgclKefM0aXXnd2p5zTUEe99T\nlAtXHuprRwxtSQUzHxJCdGlUGRGRR2aS9W984SNDVmcegnOIrZD2pVm/tjDwVex5\nMuAuKHr8et1EKyvKCnta6USq7WC2l6RdsCaAYzSTQ7ljEi9A+6Q=\n=/9g0\n-----END PGP SIGNATURE-----\n".to_string(),
        };

        tag.meta = tag.encode_metadata().unwrap();
        assert_eq!("e5c324b03b72b26f11557c4955c6d17c68dc8595", tag.meta.id.to_string());

        tag.write_to_file("/tmp".to_string()).expect("Write error!");
        assert!(Path::new("/tmp/e5/c324b03b72b26f11557c4955c6d17c68dc8595").exists());

    }

    #[test]
    fn test_idx_read_from_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/pack-8d36a6464e1f284e5e9d06683689ee751d4b2687.idx");
        let f = File::open(path).ok();
        let mut reader = BufReader::new(f.unwrap());
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).ok();

        let mut idx = super::Idx {
            version: 0,
            number_of_objects: 0,
            map_of_prefix: HashMap::new(),
            idx_items: Vec::new(),
            pack_signature: super::ID { bytes: vec![], hash: "".to_string() },
            idx_signature: super::ID { bytes: vec![], hash: "".to_string() },
        };

        idx.decode(buffer).unwrap();

        assert_eq!(2, idx.version);
        assert_eq!(614, idx.number_of_objects);
        assert_eq!(2, idx.map_of_prefix["7c"]);
        assert_eq!(idx.number_of_objects, idx.idx_items.len());
        assert_eq!("8d36a6464e1f284e5e9d06683689ee751d4b2687", idx.pack_signature.to_string());
        assert_eq!("92d07408a070a5fbea3c1f2d00e696293b78e7c6", idx.idx_signature.to_string());
    }

    #[test]
    fn test_pack_read_from_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/pack-8d36a6464e1f284e5e9d06683689ee751d4b2687.pack");
        let f = File::open(path).ok();
        let mut reader = BufReader::new(f.unwrap());
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).ok();

        let mut pack = super::Pack {
            head: "".to_string(),
            version: 0,
            number_of_objects: 0,
            signature: super::ID { bytes: vec![], hash: "".to_string() },
        };

        pack.decode(buffer).unwrap();

        assert_eq!("PACK", pack.head);
        assert_eq!(2, pack.version);
        assert_eq!("8d36a6464e1f284e5e9d06683689ee751d4b2687", pack.signature.to_string());
    }

    #[test]
    fn test_idx_write_to_file() {

    }

    #[test]
    fn test_pack_write_to_file() {

    }
}