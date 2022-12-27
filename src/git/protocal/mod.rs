pub mod http_protocol;

// #[derive(Default)]
pub struct HttpProtocol {
    pub mode: AckMode,
}
impl Default for HttpProtocol {
    fn default() -> Self {
        Self {
            mode: AckMode::MultiAckDetailed,
        }
    }
}

// #[derive(Default)]
pub enum AckMode {
    MultiAck,
    MultiAckDetailed,
    Neither,
}
impl HttpProtocol {
    pub fn value_in_ack_mode<'a>(mode: &AckMode) -> &'a str {
        match mode {
            AckMode::MultiAck => "multi_ack",
            AckMode::MultiAckDetailed => "multi_ack_detailed",
            AckMode::Neither => "",
        }
    }
}
