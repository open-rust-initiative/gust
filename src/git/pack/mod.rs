//!
//!
//!
//!
//!
//!
//!


use std::io::Read;
use std::path::Path;

use super::cache::PackObjectCache;
use super::file::*;
use super::object::delta::*;
use super::object::Object;
use crate::errors::GitError;

use std::convert::TryFrom;
use crate::git::errors::make_error;
use crate::git::id::ID;
use anyhow::Result;
use bstr::ByteSlice;
use byteorder::{BigEndian, ReadBytesExt};
use flate2::read::ZlibDecoder;
use std::fs::File;
use std::rc::Rc;

/// #### Pack文件结构<br>
///  `head`: always = "PACK" <br>
/// `version`: version code <br>
/// `number_of_objects` : Total mount of objects <br>
/// `signature`:ID
#[allow(unused)]
pub struct Pack {
    head: String,
    version: u32,
    number_of_objects: u32,
    signature: ID,
}

///
impl Pack {
    /// Git [Pack Format](https://github.com/git/git/blob/master/Documentation/technical/pack-format.txt)
    #[allow(unused)]
    pub fn decode() {
        let mut pack_file  = File::open(&Path::new(
            //".git/objects/aa/36c1e0d709f96d7b356967e16766bafdf63a75",
            "./resources/data/test/pack-6590ba86f4e863e1c2c985b046e1d2f1a78a0089.pack",
        ))
        .unwrap();
        let magic = read_bytes(&mut pack_file).unwrap();

        if magic != *b"PACK" {
             panic!("not stand pack file");
        }
      
        let version = read_u32(&mut pack_file).unwrap();
        if version != 2 {
            panic!("not support version");
        }
      
        let total_objects = read_u32(&mut pack_file).unwrap();
        let mut object_cache = PackObjectCache::default();
        let mut first_byte_objects = [0u32; 1 << u8::BITS];
        let mut object_offsets = Vec::with_capacity(total_objects as usize);
        for _ in 0..total_objects {
          let offset = get_offset(&mut pack_file).unwrap();
          let object = Pack::read_pack_object(&mut pack_file, offset, &mut object_cache).unwrap();
          println!("****************************" );
          println!("hash :{}",object.hash() );
          println!("{}",object.contents.len());
          println!("{}", object.contents);

          let object_hash = object.hash();
          first_byte_objects[object_hash.0[0] as usize] += 1;
          // Larger offsets would require a version-2 pack index
          let offset = u32::try_from(offset).map_err(|_| {
            make_error("Packfile is too large")
          }).unwrap();
          object_offsets.push((object_hash, offset));
        }
    }
    ///GetObject  
    pub fn read_pack_object(
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
                let base_object = if let Some(object) = cache.Offset_object(base_offset) {
                    Rc::clone(object)
                } else {
                    Pack::read_pack_object(pack_file, base_offset, cache)?
                };
                seek(pack_file, offset)?;
                let objs = apply_delta(pack_file, &base_object)?;
                Ok(objs)
                
            }
            // Deltified; base object is given by a hash outside the packfile
            Some(HashDelta) => {
                let hash = read_hash(pack_file)?;
                let object;
                let base_object = if let Some(object) = cache.Hash_object(hash) {
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
    ///
    #[allow(unused)]
    fn next_object(&self, data: &mut Vec<u8>, index: &mut usize) -> Result<usize, GitError> {
        let mut offset = *index;
        let mut byte = data[offset];
        offset += 1;
        let object_type = (byte & 0x70) >> 4; // 0111
        let mut _object_size = (byte & 0xf) as u64; //0000 1111
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
                let mut data_string = Vec::new();
                deflate_stream.read_to_end(&mut data_string)?;
                offset += deflate_stream.total_in() as usize;

                let mut sss = String::from_utf8(data_string).expect("errpr");

                println!("{}", sss);
                println!("************");

                Ok(offset)
            }
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
            }
            7 => {
                // REF_DELTA 偏移
                offset += 20;

                let mut deflate_stream = ZlibDecoder::new(&data[offset..]);
                let mut instructions = Vec::new();
                deflate_stream.read_to_end(&mut instructions)?;
                offset += deflate_stream.total_in() as usize;

                Ok(offset)
            }
            _ => Err(GitError::InvalidObjectType(object_type.to_string())),
        }
    }
}

///
#[cfg(test)]
mod tests {
    use crate::git::cache::PackObjectCache;
    use crate::git::id::ID;
    use std::env;
    use std::fs::File;
    use std::io::BufReader;
    use std::io::Read;
    use std::path::Path;
    use std::path::PathBuf;

    use super::Pack;

    ///
    #[test]
    fn test_pack_read_from_file() {
        //let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        //path.push("resources/data/test/pack-8d36a6464e1f284e5e9d06683689ee751d4b2687.pack");

        let f = File::open(&Path::new(
            //".git/objects/aa/36c1e0d709f96d7b356967e16766bafdf63a75",
            "./resources/data/test/pack-6590ba86f4e863e1c2c985b046e1d2f1a78a0089.pack",
        ))
        .unwrap();
        //let f = File::open(path).ok();
        let mut reader = BufReader::new(f);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).ok();

        let mut pack = Pack {
            head: "".to_string(),
            version: 0,
            number_of_objects: 0,
            signature: ID {
                bytes: vec![],
                hash: "".to_string(),
            },
        };

        //pack.decode(buffer).unwrap();

        assert_eq!("PACK", pack.head);
        assert_eq!(2, pack.version);
        assert_eq!(
            "6590ba86f4e863e1c2c985b046e1d2f1a78a0089",
            pack.signature.to_string()
        );
    }

    ///
    #[test]
    fn test_pack_write_to_file() {
      Pack::decode();
    }
}
