//!
//!
//!
//!
//!
//!
//!

use std::fmt::Display;

/// Git Object hash type. only support SHA1 for now.
#[allow(unused)]
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
pub enum Hash {
    Sha1,
}

/// Display trait for Hash type
impl Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Hash::Sha1 => write!(f, "sha1"),
        }
    }
}

mod tests {
    use super::Hash;

    #[test]
    fn test_hash() {
        assert_eq!(Hash::Sha1.to_string(), "sha1");
    }
}