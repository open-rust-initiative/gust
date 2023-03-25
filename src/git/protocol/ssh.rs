//!
//!
//!
//!

use async_trait::async_trait;
use russh::server::{Auth, Msg, Session};
use russh::*;
use russh_keys::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::gust::driver::ObjectStorage;

use super::HttpProtocol;

#[derive(Clone)]
pub struct SshServer<T> {
    pub client_pubkey: Arc<russh_keys::key::PublicKey>,
    pub clients: Arc<Mutex<HashMap<(usize, ChannelId), Channel<Msg>>>>,
    pub id: usize,
    pub storage: T,
}

impl<T> server::Server for SshServer<T>
where
    T: ObjectStorage + Send + Sync + Clone,
{
    type Handler = Self;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self {
        let s = self.clone();
        self.id += 1;
        s
    }
}

#[async_trait]
impl<T> server::Handler for SshServer<T>
where
    T: ObjectStorage + Send + Sync + Clone,
{
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
        _channel: ChannelId,
        data: &[u8],
        mut session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        tracing::info!("data: {:?}", String::from_utf8(data.to_vec()).unwrap());
        {
            let mut clients = self.clients.lock().unwrap();
            for ((id, _channel_id), ref mut channel) in clients.iter_mut() {
                channel.data(data);
            }
        }
        Ok((self, session))
    }
}

impl<T> SshServer<T>
where
    T: ObjectStorage + Send + Sync + Clone,
{
    async fn handle_git_command(&self, command: &str) -> String {
        let command: Vec<_> = command.split(' ').collect();
        let mut http_protocol = HttpProtocol::new(
            PathBuf::from(command[1]),
            command[0],
            Arc::new(self.storage.clone()),
        );
        let res = http_protocol.git_info_refs().await;
        String::from_utf8(res.to_vec()).unwrap()
    }
}
