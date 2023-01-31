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
}

///
impl Default for HttpProtocol {
    fn default() -> Self {
        Self {
            mode: AckMode::MultiAckDetailed,
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
