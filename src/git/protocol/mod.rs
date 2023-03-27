//!
//!
//!
//!

use std::{fs::File, path::PathBuf, sync::Arc, clone};

use clap::Subcommand;

use crate::{git::protocol::pack::SP, gust::driver::ObjectStorage};

use super::pack::Pack;
pub mod http;
pub mod pack;
pub mod ssh;

#[derive(Debug, Clone)]
pub struct PackProtocol<T: ObjectStorage> {
    pub protocol: Protocol,
    pub capabilities: Vec<Capability>,
    pub path: PathBuf,
    pub service_type: Option<ServiceType>,
    pub storage: Arc<T>,
}

// Is that useful?
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Protocol {
    Local,
    Http,
    Ssh,
    Git,
}

#[derive(Debug, PartialEq, Clone, Copy)]
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


// TODO: Additional Capabilitys need to be added.
#[derive(Debug, Clone, PartialEq)]
pub enum Capability {
    MultiAck,
    MultiAckDetailed,
    SideBand,
    SideBand64k,
    ReportStatus,
    ReportStatusv2,
    OfsDelta,
    DeepenSince,
    DeepenNot,
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
pub struct RefUpdateRequet {
    pub comand_list: Vec<RefCommand>,
}

#[derive(Debug, Clone)]
pub struct RefCommand {
    pub ref_name: String,
    pub old_id: String,
    pub new_id: String,
    pub status: String,
    pub error_msg: String,
}

impl RefCommand {
    const OK_STATUS: &str = "ok";

    const FAILED_STATUS: &str = "ng";

    pub fn new(old_id: String, new_id: String, ref_name: String) -> Self {
        RefCommand {
            ref_name,
            old_id,
            new_id,
            status: RefCommand::OK_STATUS.to_owned(),
            error_msg: "".to_owned(),
        }
    }

    pub fn unpack(&mut self, pack_file: &mut File) -> Result<Pack, anyhow::Error> {
        match Pack::decode(pack_file) {
            Ok(decoded_pack) => {
                self.status = RefCommand::OK_STATUS.to_owned();
                Ok(decoded_pack)
            }
            Err(err) => {
                self.status = RefCommand::FAILED_STATUS.to_owned();
                self.error_msg = err.to_string();
                Err(err.into())
            }
        }
    }

    pub fn get_status(&self) -> String {
        if RefCommand::OK_STATUS == self.status {
            format!("{}{}{}", self.status, SP, self.ref_name,)
        } else {
            format!(
                "{}{}{}{}{}",
                self.status,
                SP,
                self.ref_name,
                SP,
                self.error_msg.clone()
            )
        }
    }

    pub fn failed(&mut self, msg: String) {
        self.status = RefCommand::FAILED_STATUS.to_owned();
        self.error_msg = msg;
    }
}

#[allow(unused)]
impl<T: ObjectStorage> PackProtocol<T> {
    pub fn new( path: PathBuf, service_name: &str, storage: Arc<T>) -> Self {
        let service_type = if service_name.is_empty() {
            None
        } else {
            Some(ServiceType::new(service_name))
        };
        PackProtocol {
            // TODO: remove hard code here
            protocol: Protocol::Ssh,
            capabilities: Vec::new(),
            service_type,
            path,
            storage,
        }
    }

    // pub fn service_type(&mut self, service_name: &str) {
    //     self.service_type = Some(ServiceType::new(&service_name));
    // }

    // #[allow(unused)]
    // pub fn value_in_ack_mode<'a>(mode: &AckMode) -> &'a str {
    //     match mode {
    //         AckMode::MultiAck => "multi_ack",
    //         AckMode::MultiAckDetailed => "multi_ack_detailed",
    //         AckMode::Neither => "",
    //     }
    // }
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
