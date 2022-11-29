//!
//!
//!
//!
//!
//!
//!

use std::fmt::Display;
use std::io::Error;
use std::str::FromStr;
use std::convert::TryFrom;
use super::errors;
///Hash值的位数 - sha1
const HASH_BYTES: usize = 20;
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
impl Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut hashstr =String::new();
        for i in self.0{
            hashstr += (format!("{:2x}",i).as_mut_str());               
        }
        write!(f,"{}",hashstr)
    }
}



impl Hash {
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
    type Err = Error;
    fn from_str(hex_hash: &str) -> std::io::Result<Self> {
        Hash::hex_to_hash(hex_hash.as_bytes())
            .ok_or_else(|| errors::make_error(&format!("Invalid hash: {}", hex_hash)))
    }
}

mod tests {
    #[test]
    fn test_hash() {
        
    }
}
