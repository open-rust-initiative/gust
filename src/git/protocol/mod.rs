//!
//!
//!
//!

use std::path::PathBuf;

use clap::Subcommand;
pub mod http;
pub mod ssh;

pub struct HttpProtocol {
    pub mode: AckMode,
    pub repo_dir: PathBuf,
}

///
impl Default for HttpProtocol {
    fn default() -> Self {
        Self {
            mode: AckMode::MultiAckDetailed,
            repo_dir: PathBuf::new(),
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum ServiceType {
    UploadPack,
    ReceivePack,
}

impl ServiceType {
    pub fn new(service_name: &str) -> Self {
        match service_name {
            "git-upload-pack" => ServiceType::UploadPack,
            "git-receive-pack" => ServiceType::ReceivePack,
            _ => panic!("service type not supported"),
        }
    }
    pub fn to_string(&self) -> String {
        match self {
            ServiceType::UploadPack => "git-upload-pack".to_owned(),
            ServiceType::ReceivePack => "git-receive-pack".to_owned(),
        }
    }
}

///
///
///
#[allow(unused)]
pub enum AckMode {
    MultiAck,
    MultiAckDetailed,
    Neither,
}

pub enum SideBind {
    // sideband 1 will contain packfile data,
    PackfileData,
    // sideband 2 will be used for progress information that the client will generally print to stderr and
    ProgressInfo,
    // sideband 3 is used for error information.
    Error,
}

impl SideBind {
    pub fn value(&self) -> u8 {
        match self {
            Self::PackfileData => b'\x01',
            Self::ProgressInfo => b'\x02',
            Self::Error => b'\x03',
        }
    }
}

///
///
///
#[allow(unused)]
impl HttpProtocol {
    #[allow(unused)]
    pub fn value_in_ack_mode<'a>(mode: &AckMode) -> &'a str {
        match mode {
            AckMode::MultiAck => "multi_ack",
            AckMode::MultiAckDetailed => "multi_ack_detailed",
            AckMode::Neither => "",
        }
    }
}

#[derive(Subcommand)]
pub enum ServeCommand {
    Serve {
        #[arg(short, long)]
        port: Option<u16>,

        #[arg(short, long, value_name = "FILE")]
        key_path: Option<PathBuf>,

        #[arg(short, long, value_name = "FILE")]
        cert_path: Option<PathBuf>,
    },
}
