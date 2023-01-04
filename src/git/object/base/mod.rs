//!
//!
//!
//!
//!

use std::fmt::Display;

pub mod blob;
pub mod commit;
pub mod sign;
pub mod tag;
pub mod tree;

/// **The Object Class Enum**<br>
/// Merge the four basic classes into an enumeration structure for easy saving
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub enum ObjectClass {
    BLOB(blob::Blob),
    COMMIT(commit::Commit),
    TREE(tree::Tree),
    TAG(tag::Tag),
}

///
///
///
impl Display for ObjectClass {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ObjectClass::BLOB(_) => write!(f, "BLOB"),
            ObjectClass::COMMIT(_) => write!(f, "COMMIT"),
            ObjectClass::TREE(_) => write!(f, "TREE"),
            ObjectClass::TAG(_) => write!(f, "TAG"),
        }
    }
}
