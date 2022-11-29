use std::io::Error;
///四种Objct类型
#[derive(Clone, Copy, Debug)]
pub enum ObjectType {
    Commit,
    Tree,
    Blob,
    Tag,
}
///六种Objec存储类型
pub enum PackObjectType {
    Base(ObjectType),
    OffsetDelta,
    HashDelta,
}

/// 通过类型号分辨类型
pub fn typeNumber2Type(type_number: u8) -> Option<PackObjectType> {
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

pub fn type2Number(_type: Option<PackObjectType>) -> i32{
    use ObjectType::*;
    use PackObjectType::*;
    match _type {
        Some(Base(Commit)) => 1,
        Some(Base(Tree)) => 2,
        Some(Base(Blob)) => 3,
        Some(Base(Tag)) => 4,
        Some(OffsetDelta) => 6,
        Some(HashDelta) => 7,
        None => 5,
    }
}
