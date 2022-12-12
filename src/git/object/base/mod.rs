pub mod blob;
pub mod commit;
pub mod sign;
pub mod tag;
pub mod tree;
use std::fmt::Display;

pub use super::Metadata;


/// **The Object Class Enum**<br>
/// Merge the four basic classes into an enumeration structure for easy saving
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub enum ObjClass {
    BLOB(blob::Blob),
    COMMIT(commit::Commit),
    TREE(tree::Tree),
    TAG(tag::Tag),
}
impl Display for ObjClass {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ObjClass::BLOB(a) => a.fmt(f),
            ObjClass::COMMIT(b) => b.fmt(f),
            ObjClass::TREE(c) => c.fmt(f),
            ObjClass::TAG(d) => d.fmt(f),
        }
    }
}