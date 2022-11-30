//!
//!
//!
//!
//!
//!
//!

use std::collections::HashMap;
use std::fmt::Display;
use std::io::Cursor;

use byteorder::{BigEndian, ReadBytesExt};

use crate::errors::GitError;
use crate::git::id::ID;

///
#[allow(unused)]
pub struct IdxItem {
    pub id: ID,
    pub crc32: String,
    pub offset: usize,
}

///
impl Display for IdxItem {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} {} ({})", self.offset, self.id, self.crc32)
    }
}

///
#[allow(unused)]
pub struct Idx {
    pub version: u32,
    pub number_of_objects: usize,
    pub map_of_prefix: HashMap<String, usize>,
    pub idx_items: Vec<IdxItem>,
    pub pack_signature: ID,
    pub idx_signature: ID,
}

///
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
    pub fn decode(& mut self, data: Vec<u8>) -> Result<(), GitError> {
        let mut offset : usize = 0;

        let mut id_of_objects: Vec<ID> = Vec::new();
        let mut crc32_of_objects: Vec<String> = Vec::new();

        // 4-byte Header: //FF 74 4F 63
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
            let id = ID::from_bytes(&data[(i as usize)..(i as usize) + 20]);
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

///
#[cfg(test)]
mod tests {
    use std::env;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::BufReader;
    use std::io::Read;
    use std::path::PathBuf;

    use crate::git::id::ID;

    use super::Idx;

    ///
    #[test]
    fn test_idx_read_from_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/pack-8d36a6464e1f284e5e9d06683689ee751d4b2687.idx");

        let f = File::open(path).ok();
        let mut reader = BufReader::new(f.unwrap());
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

        assert_eq!(2, idx.version);
        assert_eq!(614, idx.number_of_objects);
        assert_eq!(2, idx.map_of_prefix["7c"]);
        assert_eq!(idx.number_of_objects, idx.idx_items.len());
        assert_eq!("8d36a6464e1f284e5e9d06683689ee751d4b2687", idx.pack_signature.to_string());
        assert_eq!("92d07408a070a5fbea3c1f2d00e696293b78e7c6", idx.idx_signature.to_string());
    }

    ///
    #[test]
    fn test_idx_write_to_file() {

    }
}