use thiserror::Error;
use crate::git::hash::Hash;

#[derive(Error, Debug)]
pub enum GitError {
    #[error("The `{0}` is not a valid git object type.")]
    InvalidObjectType(String),

    #[error("The `{0}` is not a valid idx file.")]
    InvalidIdxFile(String),

    #[error("The `{0}` is not a valid pack file.")]
    InvalidPackFile(String),

    #[error("The `{0}` is not a valid pack header.")]
    InvalidPackHeader(String),

    #[error("The `{0}` is not a valid git tree type.")]
    InvalidTreeItem(String),

    #[error("The {0} is not a valid Hash value ")]
    InvalidHashValue(String),

    #[error("Delta Object Error Info:{0}")]
    DeltaObjError(String),

    #[error("The object to be packed is incomplete ,{0}")]
    UnCompletedPackObject(String),

    #[error("Error decode in the Object ,info:{0}")]
    InvalidObjectInfo(String),

    #[error("Can't found Hash value :{0} from current file")]
    NotFountHashValue(Hash),

    #[error(transparent)]
    IOError(#[from] std::io::Error),
    


}