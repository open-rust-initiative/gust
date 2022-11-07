//!
//!
//!
//!
//!
//!
//!

use std::io::Read;
use std::io::Cursor;

use byteorder::{BigEndian, ReadBytesExt};
use bstr::ByteSlice;
use flate2::read::ZlibDecoder;
use anyhow::Result;

use crate::errors::GitError;
use crate::git::id::ID;

///
#[allow(unused)]
struct Pack {
    head: String,
    version: u32,
    number_of_objects: u32,
    signature: ID,
}

///
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

    ///
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
#[cfg(test)]
mod tests {
    use std::env;
    use std::fs::File;
    use std::io::BufReader;
    use std::io::Read;
    use std::path::PathBuf;

    ///
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

    ///
    #[test]
    fn test_pack_write_to_file() {

    }
}