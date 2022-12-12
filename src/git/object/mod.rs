//!Object struct , contain the raw info cut from the pack file or other file
//! 

use types::ObjectType;
use super::hash::Hash;

pub mod types;
pub mod delta;
pub mod base;
mod metadata;
pub use metadata::Metadata;


//Object内存存储类型 
///# Deprecate
#[derive(Clone, Debug)]
pub struct Object {
  pub object_type: ObjectType,
  pub contents: Vec<u8>,
}
#[allow(dead_code)]
impl Object {
    /// object 的 hash转化函数
    pub fn hash(&self) -> Hash {
      Hash::from_meta(&self.to_metadata())
    }
   // pub fn GetObjectFromPack()
    pub fn to_metadata(&self) -> Metadata{
      Metadata::new(self.object_type, &self.contents)
    }
  }


#[cfg(test)]
mod tests{
    use super::Object;

  #[test] 
  fn test_obj_hash(){
    let _obj=Object{
      object_type:super::types::ObjectType::Blob,
      contents : String::from("hello ,sss").into_bytes(),
    };
    print!("{}",_obj.hash())  ;//602091219933865cace5ab8cd78b424735c82e6c

  }
}