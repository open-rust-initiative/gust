//!
//!
//!
//!
//!
//!
//!

use std::fmt::Display;
use crate::errors::GitError;
use std::convert::TryFrom;
use std::str::FromStr;
use sha1::{Digest, Sha1};
///Hash值的位数 - sha1
pub const HASH_BYTES: usize = 20;
/// Git Object hash type. only support SHA1 for now.
///
///
#[allow(unused)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HashType {
    Sha1,
}

#[allow(unused)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Hash(pub [u8; HASH_BYTES]);

/// Display trait for Hash type
use colored::Colorize;
impl Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut hash_str = String::new();
        for i in self.0 {
            
            hash_str += format!("{:1X}", i>>4 & 0x0f).as_str();
            hash_str += format!("{:1X}", i & 0x0f).as_str();
        }
        write!(f, "{}", hash_str.red().bold())
    }
}

impl Hash {
    pub fn new(data:&Vec<u8>) -> Hash{
        let new_hash = Sha1::new()
        .chain(data)
        .finalize();
      Hash(<[u8; HASH_BYTES]>::try_from(new_hash.as_slice()).unwrap())
    }
    ///解析出16进制数字0-f
    fn hex_char_value(hex_char: u8) -> Option<u8> {
        match hex_char {
            b'0'..=b'9' => Some(hex_char - b'0'),
            b'a'..=b'f' => Some(hex_char - b'a' + 10),
            _ => None,
        }
    }
    ///将u8数组转化为hash
    fn hex_to_hash(hex_hash: &[u8]) -> Option<Hash> {
        const BITS_PER_CHAR: usize = 4;
        const CHARS_PER_BYTE: usize = 8 / BITS_PER_CHAR;
        // 将切片以chunks_size的切片
        let byte_chunks = hex_hash.chunks_exact(CHARS_PER_BYTE);
        if !byte_chunks.remainder().is_empty() {
            return None;
        }
        let bytes = byte_chunks
            .map(|hex_digits| {
                hex_digits.iter().try_fold(0, |value, &byte| {
                    let char_value = Hash::hex_char_value(byte)?;
                    Some(value << BITS_PER_CHAR | char_value)
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let bytes = <[u8; HASH_BYTES]>::try_from(bytes).ok()?;
        Some(Hash(bytes))
    }
}

impl FromStr for Hash {
    type Err = GitError;
    fn from_str(hex_hash: &str) -> Result<Self, GitError> {
        Hash::hex_to_hash(hex_hash.as_bytes())
            .ok_or_else(|| GitError::InvalidHashValue(hex_hash.to_string()))
    }
}

mod tests {

    /// The Right Hash decode
    #[test]
    fn test_hash() {
        use super::Hash;
        use std::str::FromStr;
        let test_hash = Hash::from_str("18fd2deaaf152c7f1222c52fb2673f6192b375f0").unwrap();
        let result_hash: [u8; 20] = [
            24, 253, 45, 234, 175, 21, 44, 127, 18, 34, 197, 47, 178, 103, 63, 97, 146, 179, 117,
            240,
        ];
        assert_eq!(test_hash.0, result_hash);
        println!("{}",test_hash);
    }


        /// The Right Hash decode
        #[test]
        fn test_hash_with_zero() {
            use super::Hash;
            use std::str::FromStr;
            let test_hash = Hash::from_str("08fd2deaaf152c7f1222c52fb2673f6192b37500").unwrap();
            let result_hash: [u8; 20] = [
                8, 253, 45, 234, 175, 21, 44, 127, 18, 34, 197, 47, 178, 103, 63, 97, 146, 179, 117, 0
            ];
             assert_eq!(test_hash.0, result_hash);
            println!("{}",test_hash);
        }
    /// The Wrong Hash decode
    #[test]
    fn test_error_hash() {
        use super::Hash;
        use std::str::FromStr;
        let test_str = "18fd2deaaf152c7f1222c52fb2673f6192z375f0";
        let test_hash = Hash::from_str(test_str).unwrap_err();
        print!("{:?}", test_hash);
        assert_eq!(
            format!("The {} is not a valid Hash value ", test_str),
            test_hash.to_string()
        );

    }
}
