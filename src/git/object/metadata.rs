//!
//!
//!
//!

use std::fs::{create_dir_all, File};
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;

use anyhow::Context;
use bstr::ByteSlice;
use deflate::{Compression, write::ZlibEncoder};
use flate2::read::ZlibDecoder;

use crate::errors::GustError;
use crate::git::errors::GitError;
use crate::git::hash::{Hash, HashType};
use crate::git::object::types::ObjectType;

/// The metadata of git object.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct Metadata {
    pub t: ObjectType,
    pub h: HashType,
    pub id: Hash,
    pub size: usize,
    pub data: Vec<u8>,
    pub delta_header: Vec<u8>,
}

/// Implement function for Metadata
impl Metadata {
    ///
    pub fn hash(&self) -> Hash {
        Hash::from_meta(&self)
    }

    ///
    pub fn new(object_type: ObjectType, data: &Vec<u8>) -> Metadata {
        let mut metadata = Metadata {
            t: object_type,
            h: HashType::Sha1,
            id: Hash::default(),
            size: data.len(),
            data: data.to_vec(),
            delta_header: vec![],
        };

        // compute hash value
        metadata.id = metadata.hash();

        metadata
    }

    /// Write the object to the file system with folder and file.
    /// This function can create a “loose” object format,
    /// which can convert into the `.pack` format by the Command:
    /// ```bash
    ///     git gc
    /// ```
    #[allow(unused)]
    pub(crate) fn write_to_file(&self, root_path: String) -> Result<String, GitError> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::Default);

        encoder.write_all(&self.t.to_bytes());
        encoder.write(&[b' ']);
        encoder.write(self.data.len().to_string().as_bytes());
        encoder.write(&[b'\0']);
        encoder.write_all(&self.data).expect("Write error!");
        let compressed_data = encoder.finish().expect("Failed to finish compression!");

        let mut path = PathBuf::from(root_path);
        path.push(&self.id.to_folder());
        create_dir_all(&path)
            .with_context(|| format!("Failed to create directory: {}", path.display()))
            .unwrap();

        path.push(&self.id.to_filename());

        let mut file = File::create(&path)
            .with_context(|| format!("Failed to create file: {}", path.display()))
            .unwrap();
        file.write_all(&compressed_data)
            .with_context(|| format!("Failed to write to file: {}", path.display()))
            .unwrap();

        Ok(path.to_str().unwrap().to_string())
    }

    ///Convert Metadata to the Vec<u8> ,so that it can write to File
    pub fn convert_to_vec(&self) -> Result<Vec<u8>, GustError> {
        let mut compressed_data =
            vec![(0x80 | (self.t.type2_number() << 4)) + (self.size & 0x0f) as u8];
        let mut _size = self.size >> 4;
        if _size > 0 {
            while _size > 0 {
                if _size >> 7 > 0 {
                    compressed_data.push((0x80 | _size) as u8);
                    _size >>= 7;
                } else {
                    compressed_data.push((_size) as u8);
                    break;
                }
            }
        } else {
            compressed_data.push(0);
        }

        match self.t {
            ObjectType::OffsetDelta => {
                compressed_data.append(&mut self.delta_header.clone());
            }
            ObjectType::HashDelta => {
                compressed_data.append(&mut self.delta_header.clone());
            }
            _ => {}
        }

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::Default);
        encoder.write_all(&self.data).expect("Write error!");
        compressed_data.append(&mut encoder.finish().expect("Failed to finish compression!"));

        Ok(compressed_data)
    }

    /// Read the object from the file system and parse to a metadata object.<br>
    /// This file is the “loose” object format.
    #[allow(unused)]
    pub(crate) fn read_object_from_file(path: String) -> Result<Metadata, GitError> {
        let file = File::open(path).unwrap();
        let mut reader = BufReader::new(file);
        let mut data = Vec::new();
        reader.read_to_end(&mut data).unwrap();

        let mut decoder = ZlibDecoder::new(&data[..]);
        let mut decoded = Vec::new();
        decoder.read_to_end(&mut decoded).unwrap();

        let type_index = decoded.find_byte(0x20).unwrap();
        let t = &decoded[0..type_index];

        let size_index = decoded.find_byte(0x00).unwrap();
        let size = decoded[type_index + 1..size_index]
            .iter()
            .copied()
            .map(|x| x as char)
            .collect::<String>()
            .parse::<usize>()
            .unwrap();

        let mut data = decoded[size_index + 1..].to_vec();

        match String::from_utf8(t.to_vec()).unwrap().as_str() {
            "blob" => Ok(Metadata::new(ObjectType::Blob, &data)),
            "tree" => Ok(Metadata::new(ObjectType::Tree, &data)),
            "commit" => Ok(Metadata::new(ObjectType::Commit, &data)),
            "tag" => Ok(Metadata::new(ObjectType::Tag, &data)),
            _ => Err(GitError::InvalidObjectType(
                String::from_utf8(t.to_vec()).unwrap(),
            )),
        }
    }

    /// Change the base object to the delta object ,
    /// including : ref-object ofs-object
    pub fn change_to_delta(&mut self, types: ObjectType, changed: Vec<u8>, header: Vec<u8>) {
        self.t = types;
        self.data = changed;
        self.size = self.data.len();
        self.delta_header = header;
    }
}
