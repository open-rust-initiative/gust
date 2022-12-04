use std:: fmt::Display;

use crate::errors::GitError;
///四种Object类型
/// Git Object Types: Blob, Tree, Commit, Tag
#[derive(PartialEq, Eq, Hash, Ord, PartialOrd, Debug, Clone, Copy)]
pub enum ObjectType {
    Blob,
    Commit,
    Tag,
    Tree,
}


/// Display trait for Git objects type
impl Display for ObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ObjectType::Blob => write!(f, "blob"),
            ObjectType::Tree => write!(f, "tree"),
            ObjectType::Commit => write!(f, "commit"),
            ObjectType::Tag => write!(f, "tag"),
        }
    }
}

///
impl ObjectType {
    ///
    #[allow(unused)]
    pub fn to_bytes(self) -> Vec<u8> {
        match self {
            ObjectType::Blob => vec![0x62, 0x6c, 0x6f, 0x62],
            ObjectType::Tree => vec![0x74, 0x72, 0x65, 0x65],
            ObjectType::Commit => vec![0x63, 0x6f, 0x6d, 0x6d, 0x69, 0x74],
            ObjectType::Tag => vec![0x74, 0x61, 0x67],
        }
    }

    ///
    #[allow(unused)]
    pub fn from_string(s: &str) -> Result<ObjectType, GitError> {
        match s {
            "blob" => Ok(ObjectType::Blob),
            "tree" => Ok(ObjectType::Tree),
            "commit" => Ok(ObjectType::Commit),
            "tag" => Ok(ObjectType::Tag),
            _ => Err(GitError::InvalidObjectType(s.to_string())),
        }
    }
    pub fn type2_number(&self)->u8{
        match self {
            ObjectType::Blob=> 1,
            ObjectType::Commit=> 2,
            ObjectType::Tag=> 3,
            ObjectType::Tree=> 4,
        }
    }
}


///六种Object存储类型
#[derive(Debug)]
pub enum PackObjectType {
    Base(ObjectType),
    OffsetDelta,
    HashDelta,
}

/// 通过类型号分辨类型
pub fn type_number2_type(type_number: u8) -> Option<PackObjectType> {
    use ObjectType::*;
    use PackObjectType::*;
    match type_number {
        1 => Some(Base(Commit)),
        2 => Some(Base(Tree)),
        3 => Some(Base(Blob)),
        4 => Some(Base(Tag)),
        6 => Some(OffsetDelta),
        7 => Some(HashDelta),
        _ => None,
    }
}

// pub fn type2_number(_type: Option<PackObjectType>) -> i32{
//     use ObjectType::*;
//     use PackObjectType::*;
//     match _type {
//         Some(Base(Commit)) => 1,
//         Some(Base(Tree)) => 2,
//         Some(Base(Blob)) => 3,
//         Some(Base(Tag)) => 4,
//         Some(OffsetDelta) => 6,
//         Some(HashDelta) => 7,
//         None => 5,
//     }
// }
