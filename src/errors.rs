use thiserror::Error;


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

    #[error("Error decode in the Object ,info:{0}")]
    InvalidObjectInfo(String),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

}