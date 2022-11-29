
use types::ObjectType;
use super::hash::Hash;
use sha1::{Digest, Sha1};
use std::convert::TryFrom;
const COMMIT_OBJECT_TYPE: &[u8] = b"commit";
const TREE_OBJECT_TYPE: &[u8] = b"tree";
const BLOB_OBJECT_TYPE: &[u8] = b"blob";
const TAG_OBJECT_TYPE: &[u8] = b"tag";
const HASH_BYTES: usize = 20;

pub mod types;
pub mod delta;
//Object内存存储类型
#[derive(Clone, Debug)]
pub struct Object {
  pub object_type: ObjectType,
  pub contents: Vec<u8>,
}
impl Object {
    /// object 的hash转化函数
    pub fn hash(&self) -> Hash {
      let new_hash = Sha1::new()
        .chain(match self.object_type {
          Commit => COMMIT_OBJECT_TYPE,
          Tree => TREE_OBJECT_TYPE,
          Blob => BLOB_OBJECT_TYPE,
          Tag => TAG_OBJECT_TYPE,
        })
        .chain(b" ")
        .chain(self.contents.len().to_string())
        .chain(b"\0")
        .chain(&self.contents)
        .finalize();
      Hash(<[u8; HASH_BYTES]>::try_from(new_hash.as_slice()).unwrap())
    }
   // pub fn GetObjectFromPack()
  }

