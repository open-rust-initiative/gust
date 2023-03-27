//!
//!
//!
//!

use async_trait::async_trait;
use bytes::{BufMut, Bytes, BytesMut};
use russh::server::{Auth, Msg, Session};
use russh::*;
use russh_keys::*;
use std::collections::HashMap;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, BufReader};

use crate::gust::driver::ObjectStorage;

use super::pack::{self};
use super::PackProtocol;

#[derive(Clone)]
pub struct SshServer<T: ObjectStorage> {
    pub client_pubkey: Arc<russh_keys::key::PublicKey>,
    pub clients: Arc<Mutex<HashMap<(usize, ChannelId), Channel<Msg>>>>,
    pub id: usize,
    pub storage: T,
}

impl<T: ObjectStorage> server::Server for SshServer<T> {
    type Handler = Self;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self {
        let s = self.clone();
        self.id += 1;
        s
    }
}

#[async_trait]
impl<T: ObjectStorage> server::Handler for SshServer<T> {
    type Error = anyhow::Error;

    async fn channel_open_session(
        self,
        channel: Channel<Msg>,
        session: Session,
    ) -> Result<(Self, bool, Session), Self::Error> {
        {
            let mut clients = self.clients.lock().unwrap();
            clients.insert((self.id, channel.id()), channel);
        }
        Ok((self, true, session))
    }

    async fn exec_request(
        self,
        channel: ChannelId,
        data: &[u8],
        mut session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        let data = String::from_utf8_lossy(data).trim().to_owned();
        tracing::info!("exec: {:?},{}", channel, data);
        let res = self.handle_git_command(&data).await;
        session.data(channel, res.into());
        Ok((self, session))
    }

    async fn auth_publickey(
        self,
        user: &str,
        public_key: &key::PublicKey,
    ) -> Result<(Self, Auth), Self::Error> {
        tracing::info!("auth_publickey: {} / {:?}", user, public_key);
        Ok((self, server::Auth::Accept))
    }

    async fn auth_password(self, user: &str, password: &str) -> Result<(Self, Auth), Self::Error> {
        tracing::info!("auth_password: {} / {}", user, password);
        // in this example implementation, any username/password combination is accepted
        Ok((self, server::Auth::Accept))
    }

    async fn data(
        self,
        channel: ChannelId,
        data: &[u8],
        mut session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        let data_str = String::from_utf8_lossy(data).trim().to_owned();
        tracing::info!("data: {:?}, channel:{}", data_str, channel);
        let (send_pack_data, buf, pack_protocol) = self.handle_fetch_pack(data).await;
        tracing::info!("buf is {:?}", buf);
        session.data(channel, String::from_utf8(buf.to_vec()).unwrap().into());

        let mut reader = BufReader::new(send_pack_data.as_slice());
        loop {
            let mut temp = BytesMut::new();
            let length = reader.read_buf(&mut temp).await.unwrap();
            if temp.is_empty() {
                let mut bytes_out = BytesMut::new();
                bytes_out.put_slice(pack::PKT_LINE_END_MARKER);
                session.data(channel, bytes_out.to_vec().into());
                tracing::info!("close session");
                return Ok((self, session));
            }
            let bytes_out = pack_protocol.build_side_band_format(temp, length);
            tracing::info!("send: bytes_out: {:?}", bytes_out.clone().freeze());
            session.data(channel, bytes_out.to_vec().into());
        }
    }
}

impl<T: ObjectStorage> SshServer<T> {
    async fn handle_git_command(&self, command: &str) -> String {
        let command: Vec<_> = command.split(' ').collect();
        // example '/root/repotest/src.git'
        let path = command[1];
        let end = path.len() - ".git'".len();
        let mut pack_protocol = PackProtocol::new(
            PathBuf::from(&path[2..end]),
            command[0],
            Arc::new(self.storage.clone()),
        );
        let res = pack_protocol.git_info_refs().await;
        String::from_utf8(res.to_vec()).unwrap()
    }

    async fn handle_fetch_pack(&self, data: &[u8]) -> (Vec<u8>, BytesMut, PackProtocol<T>) {
        // TODO: replace hard code here
        let pack_protocol = PackProtocol::new(
            PathBuf::from("root/repotest/src"),
            "",
            Arc::new(self.storage.clone()),
        );
        let (send_pack_data, buf) = pack_protocol
            .git_upload_pack(&mut Bytes::copy_from_slice(data))
            .await
            .unwrap();
        (send_pack_data, buf, pack_protocol)
    }
}
