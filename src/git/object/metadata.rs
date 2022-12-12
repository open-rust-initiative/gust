use std::fs::File;
use std::fs::create_dir_all;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use crate::errors::GitError;
use crate::git::hash::HashType;
use crate::git::object::Object;
use anyhow::Context;
use bstr::ByteSlice;
use deflate::Compression;
use deflate::write::ZlibEncoder;
use flate2::read::ZlibDecoder;
use super::Hash;
use super::ObjectType;

/// The metadata of git object.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct Metadata {
    pub t: ObjectType,
    pub h: HashType,
    pub id:Hash,
    pub size: usize,
    pub data: Vec<u8>,
}

/// Implement function for Metadata
impl Metadata {


    pub fn hash(&self) -> Hash {
        Hash::from_meta(&self)
    }
    pub fn new(obj_type:ObjectType, data:&Vec<u8>) -> Metadata{
        let mut _metadata = Metadata{
            t: obj_type,
            h: HashType::Sha1,
            id:Hash::default(),
            size: data.len(),
            data: data.to_vec(),
        };
        // compute hash value
        _metadata.id = _metadata.hash();
        _metadata
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
    pub fn convert_to_vec(&self) -> Result<Vec<u8>, GitError> {
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

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::Default);
        encoder.write_all(&self.data).expect("Write error!");
        compressed_data.append(&mut encoder.finish().expect("Failed to finish compression!"));
        Ok(compressed_data)
    }

    /// Read the object from the file system and parse to a metadata object.<br>
    /// This file is the “loose” object format.
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
        let size = decoded[type_index + 1..size_index]
            .iter()
            .copied()
            .map(|x| x as char)
            .collect::<String>()
            .parse::<usize>()
            .unwrap();

        let mut data = decoded[size_index + 1..].to_vec();

        match String::from_utf8(t.to_vec()).unwrap().as_str() {
            "blob" => Ok(Metadata {
                t: ObjectType::Blob,
                h: HashType::Sha1,
                id: Object {
                    object_type: ObjectType::Blob,
                    contents: data.clone(),
                }
                .hash(),
                size,
                data,
            }),
            "tree" => Ok(Metadata {
                t: ObjectType::Tree,
                h: HashType::Sha1,

                id: Object {
                    object_type: ObjectType::Tree,
                    contents: data.clone(),
                }
                .hash(),

                size,
                data,
            }),
            "commit" => Ok(Metadata {
                t: ObjectType::Commit,
                h: HashType::Sha1,
                id: Object {
                    object_type: ObjectType::Commit,
                    contents: data.clone(),
                }
                .hash(),
                size,
                data,
            }),
            "tag" => Ok(Metadata {
                t: ObjectType::Tag,
                h: HashType::Sha1,
                id: Object {
                    object_type: ObjectType::Tag,
                    contents: data.clone(),
                }
                .hash(),
                size,
                data,
            }),
            _ => Err(GitError::InvalidObjectType(
                String::from_utf8(t.to_vec()).unwrap(),
            )),
        }
    }


}
