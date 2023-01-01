//!
//!
//!
//!
use std::{fs::File, sync::Arc};
use std::cmp::Ordering;
use std::convert::TryFrom;

use crate::git::errors::GitError;
use crate::git::hash::Hash;
use crate::git::utils;

use super::{cache::PackObjectCache, Pack};

impl Eq for Pack {}

impl Ord for Pack {
    fn cmp(&self, other: &Self) -> Ordering {
        let a = self.pack_file.metadata().unwrap().created().unwrap();
        let b = other.pack_file.metadata().unwrap().created().unwrap();
        if a == b {
            return Ordering::Equal;
        } else if a > b {
            return Ordering::Greater;
        } else {
            return Ordering::Less;
        }
    }
}

impl PartialOrd for Pack {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Pack {
    fn eq(&self, other: &Self) -> bool {
        let a = self.pack_file.metadata().unwrap().created().unwrap();
        let b = other.pack_file.metadata().unwrap().created().unwrap();
        a == b
    }
}

impl Pack {
    #[allow(unused)]
    pub fn decode_with_cache(&self, cache: &mut PackObjectCache) -> Result<Self, GitError> {
        let mut pack_file = File::open(self.pack_file.clone()).unwrap();
        // Check the Header of Pack File
        let mut _pack = Self::check_header(&mut pack_file).unwrap();

        for _ in 0.._pack.number_of_objects {
            //update offset of the Object
            let offset = utils::get_offset(&mut pack_file).unwrap();
            //Get the next Object by the Pack::next_object() func
            let object = Pack::next_object(&mut pack_file, offset, cache).unwrap();
            // Larger offsets would require a version-2 pack index
            let offset = u32::try_from(offset)
                .map_err(|_| GitError::InvalidObjectInfo(format!("Packfile is too large")))
                .unwrap();
        }

        // CheckSum sha-1
        let _id: [u8; 20] = utils::read_bytes(&mut pack_file).unwrap();
        _pack.signature = Hash::from_row(&_id[..]);
        print!("{}", cache.by_hash.len());
        Ok(_pack)
    }

    #[allow(dead_code)]
    pub fn multi_decode(root: &str) -> Result<Self, GitError> {
        let mut total_pack = Self::default();
        total_pack.number_of_objects = 0;
        let (files, _hash_vec) = utils::find_all_pack_file(root);
        let mut pack_vec = vec![];
        for _file_ in files.iter() {
            let mut _pack = Pack::default();
            _pack.pack_file = _file_.clone();
            pack_vec.push(_pack);
        }
        pack_vec.sort();
        let mut cache = PackObjectCache::default();
        for _pack_ in pack_vec.iter_mut() {
            _pack_.decode_with_cache(&mut cache)?;
            total_pack.number_of_objects += _pack_.number_of_objects;
        }
        total_pack.result = Arc::new(cache);
        Ok(total_pack)
    }
}

#[cfg(test)]
pub mod test {}