//!
//!
//!
//!
//!
//!
//!

use std::convert::TryInto;
use std::io::Read;

use super::cache::PackObjectCache;
use super::file::*;
use super::idx::Idx;
use super::object::delta::*;
use super::object::Object;
use crate::errors::GitError;

use crate::git::errors::make_error;
use crate::git::id::ID;
use std::convert::TryFrom;
use std::fs::File;
use std::rc::Rc;
pub mod decode;
/// #### Pack文件结构<br>
///  `head`: always = "PACK" <br>
/// `version`: version code <br>
/// `number_of_objects` : Total mount of objects <br>
/// `signature`:ID
#[allow(unused)]
pub struct Pack {
    head: [u8; 4],
    version: u32,
    number_of_objects: u32,
    signature: ID,
}

///
impl Pack {
    /// Git [Pack Format](https://github.com/git/git/blob/master/Documentation/technical/pack-format.txt)
    /// Git Pack-Format [Introduce](https://git-scm.com/docs/pack-format)
    /// ## Decode the Pack File standalone
    ///  - in: pack_file: &mut File
    ///  - out: The `Pack` Struct
    pub fn decode(pack_file: &mut File) -> Self {
        let mut _pack = Self {
            head: [0, 0, 0, 0],
            version: 0,
            number_of_objects: 0,
            signature: ID {
                bytes: vec![],
                hash: "".to_string(),
            },
        };

        let magic = read_bytes(pack_file).unwrap();

        if magic != *b"PACK" {
            panic!("not stand pack file");
        }
        _pack.head = magic;

        let version = read_u32(pack_file).unwrap();
        if version != 2 {
            panic!("not support pack version");
        }
        _pack.version = version;

        let total_objects = read_u32(pack_file).unwrap();
        let mut object_cache = PackObjectCache::default();
        let mut first_byte_objects = [0u32; 1 << u8::BITS];
        let mut object_offsets = Vec::with_capacity(total_objects as usize);
        _pack.number_of_objects = total_objects;
        for _ in 0..total_objects {
            let offset = get_offset(pack_file).unwrap();
            let object = Pack::next_object(pack_file, offset, &mut object_cache).unwrap();
            //获取取出的object的Hash值
            let object_hash = object.hash();
            first_byte_objects[object_hash.0[0] as usize] += 1;
            // Larger offsets would require a version-2 pack index
            let offset = u32::try_from(offset)
                .map_err(|_| make_error("Packfile is too large"))
                .unwrap();
            object_offsets.push((object_hash, offset));
        }

        let mut result = decode::objDecodedMap::default();
        result.update_from_cache(&mut object_cache);
        for (key, value) in result._map_hash.iter() {
            println!("*********************");
            println!("Hash :{}", key);
            println!("{}", value);
        }

        //_pack.signature = ID::from_bytes(&pack_file[pack_file.len() - 20..pack_file.len()]);
        let offset = get_offset(pack_file).unwrap();
        let _id: [u8; 20] = read_bytes(pack_file).unwrap();
        _pack.signature = ID::from_bytes(&_id[..]);
        //return
        _pack
    }

    pub fn decode_by_idx(idx: &mut Idx, pack_file: &mut File) -> Self {
        let mut _pack = Self {
            head: [0, 0, 0, 0],
            version: 0,
            number_of_objects: 0,
            signature: ID {
                bytes: vec![],
                hash: "".to_string(),
            },
        };
        let magic = read_bytes(pack_file).unwrap();
        if magic != *b"PACK" {
            panic!("not stand pack file");
        }
        _pack.head = magic;
        let version = read_u32(pack_file).unwrap();
        if version != 2 {
            panic!("not support pack version");
        }
        _pack.version = version;

        let total_objects = idx.number_of_objects;
        _pack.number_of_objects = u32::try_from(total_objects)
            .map_err(|_| make_error("Packfile is too large"))
            .unwrap();
        let mut cache = PackObjectCache::default();

        for idx_item in idx.idx_items.iter(){
            Pack::next_object(pack_file, idx_item.offset.try_into().unwrap(), &mut cache).unwrap();
        };

        let mut result = decode::objDecodedMap::default();
        result.update_from_cache(&mut cache);
        for (key, value) in result._map_hash.iter() {
            println!("*********************");
            println!("Hash :{}", key);
            println!("{}", value);
        }
        
        
      
        _pack.signature = idx.pack_signature.clone();
       
        _pack
    }
    ///Get the Object from File by the Give Offset
    /// by the way , the cache can hold the fount object
    pub fn next_object(
        pack_file: &mut File,
        offset: u64,
        cache: &mut PackObjectCache,
    ) -> std::io::Result<Rc<Object>> {
        use super::object::types::PackObjectType::*;
        seek(pack_file, offset)?;
        let (object_type, size) = read_type_and_size(pack_file)?;
        let object_type = super::object::types::typeNumber2Type(object_type);
        let object = match object_type {
            // Undeltified representation
            Some(Base(object_type)) => read_zlib_stream_exact(pack_file, |decompressed| {
                let mut contents = Vec::with_capacity(size);
                decompressed.read_to_end(&mut contents)?;
                if contents.len() != size {
                    return Err(make_error("Incorrect object size"));
                }

                Ok(Object {
                    object_type,
                    contents,
                })
            }),
            // Deltified; base object is at an offset in the same packfile
            Some(OffsetDelta) => {
                let delta_offset = read_offset_encoding(pack_file)?;
                let base_offset = offset
                    .checked_sub(delta_offset)
                    .ok_or_else(|| make_error("Invalid OffsetDelta offset"))?;
                let offset = get_offset(pack_file)?;
                let base_object = if let Some(object) = cache.offset_object(base_offset) {
                    Rc::clone(object)
                } else {
                    Pack::next_object(pack_file, base_offset, cache)?
                };
                seek(pack_file, offset)?;
                let objs = apply_delta(pack_file, &base_object)?;
                Ok(objs)
            }
            // Deltified; base object is given by a hash outside the packfile
            Some(HashDelta) => {
                let hash = read_hash(pack_file)?;
                let object;
                let base_object = if let Some(object) = cache.hash_object(hash) {
                    object
                } else {
                    object = read_object(hash)?;
                    &object
                };
                apply_delta(pack_file, &base_object)
            }
            None => return Err(make_error("Unkonw type of the Object!!")),
        }?;
        let obj = Rc::new(object);
        cache.update(obj.clone(), offset);
        Ok(obj)
    }
}

///
#[cfg(test)]
mod tests {
    use crate::git::id::ID;
    use crate::git::idx::Idx;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::BufReader;
    use std::io::Read;
    use std::path::Path;

    use super::Pack;

    /// Test the pack File decode standalone
    #[test]
    fn test_pack_write_to_file1() {
        let mut pack_file = File::open(&Path::new(
            "./resources/data/test/pack-6590ba86f4e863e1c2c985b046e1d2f1a78a0089.pack",
        ))
        .unwrap();
        let decoded_pack = Pack::decode(&mut pack_file);
        assert_eq!(*b"PACK", decoded_pack.head);
        assert_eq!(2, decoded_pack.version);
        assert_eq!(
            "6590ba86f4e863e1c2c985b046e1d2f1a78a0089",
            decoded_pack.signature.to_string()
        );
    }

    #[test]
    fn test_pack_write_to_file2() {
        let mut pack_file = File::open(&Path::new(
            "./resources/data/test/pack-8d36a6464e1f284e5e9d06683689ee751d4b2687.pack",
        ))
        .unwrap();
        let decoded_pack = Pack::decode(&mut pack_file);
        assert_eq!(*b"PACK", decoded_pack.head);
        assert_eq!(2, decoded_pack.version);
        assert_eq!(
            "8d36a6464e1f284e5e9d06683689ee751d4b2687",
            decoded_pack.signature.to_string()
        );
    }
    
    
    #[test]
    fn test_parse_simple_pack() {
        let mut pack_file = File::open(&Path::new(
            //".git/objects/aa/36c1e0d709f96d7b356967e16766bafdf63a75",
            "./resources/test1/pack-1d0e6c14760c956c173ede71cb28f33d921e232f.pack",
        ))
        .unwrap();
        let decoded_pack = Pack::decode(&mut pack_file);
        assert_eq!(*b"PACK", decoded_pack.head);
        assert_eq!(2, decoded_pack.version);
        assert_eq!(
            "1d0e6c14760c956c173ede71cb28f33d921e232f",
            decoded_pack.signature.to_string()
        );
    }
    
    ///Test the pack decode by the Idx File
    #[test]
    fn test_pack_idx_decode(){
        let mut pack_file = File::open(&Path::new(
            "./resources/data/test/pack-8d36a6464e1f284e5e9d06683689ee751d4b2687.pack",
        ))
        .unwrap();
        let idx_file = File::open(&Path::new(
            "./resources/data/test/pack-8d36a6464e1f284e5e9d06683689ee751d4b2687.idx",
        ))
        .unwrap();
        let mut reader = BufReader::new(idx_file);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).ok();

        let mut idx = Idx {
            version: 0,
            number_of_objects: 0,
            map_of_prefix: HashMap::new(),
            idx_items: Vec::new(),
            pack_signature: ID { bytes: vec![], hash: "".to_string() },
            idx_signature: ID { bytes: vec![], hash: "".to_string() },
        };

        idx.decode(buffer).unwrap();
        let decoded_pack = Pack::decode_by_idx(&mut idx,&mut pack_file);
        assert_eq!(*b"PACK", decoded_pack.head);
        assert_eq!(2, decoded_pack.version);
        assert_eq!(
            "8d36a6464e1f284e5e9d06683689ee751d4b2687",
            decoded_pack.signature.to_string()
        );
    }

}
