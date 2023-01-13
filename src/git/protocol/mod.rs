//!
//!
//!
//!
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
    pub fn value_in_ack_mode<'a>(mode: &AckMode) -> &'a str {
        match mode {
            AckMode::MultiAck => "multi_ack",
            AckMode::MultiAckDetailed => "multi_ack_detailed",
            AckMode::Neither => "",
        }
    }
}
