//!
//!
//!
//!
extern crate futures;
extern crate thrussh;
extern crate thrussh_keys;
extern crate tokio;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use thrussh::server::{Auth, Session};
use thrussh::*;
use thrussh_keys::*;
use tokio::runtime::Handle;

use crate::gust::driver::ObjectStorage;

use super::HttpProtocol;

#[derive(Clone)]
pub struct SshServer<T: ObjectStorage> {
    pub client_pubkey: Arc<thrussh_keys::key::PublicKey>,
    pub clients: Arc<Mutex<HashMap<(usize, ChannelId), thrussh::server::Handle>>>,
    pub id: usize,
    pub storage: T,
    pub handle: Handle,
}

impl<T> server::Server for SshServer<T>
where
    T: ObjectStorage + Send + Sync + Clone,
{
    type Handler = Self;
    fn new(&mut self, _: Option<std::net::SocketAddr>) -> Self {
        let s = self.clone();
        self.id += 1;
        s
    }
}

impl<T> server::Handler for SshServer<T>
where
    T: ObjectStorage + Send + Sync + Clone,
{
    type Error = anyhow::Error;
    type FutureAuth = futures::future::Ready<Result<(Self, server::Auth), anyhow::Error>>;
    type FutureUnit = futures::future::Ready<Result<(Self, Session), anyhow::Error>>;
    type FutureBool = futures::future::Ready<Result<(Self, Session, bool), anyhow::Error>>;

    fn finished_auth(mut self, auth: Auth) -> Self::FutureAuth {
        tracing::info!("finished_auth: {:?} ", auth);
        futures::future::ready(Ok((self, auth)))
    }

    fn finished_bool(self, b: bool, s: Session) -> Self::FutureBool {
        tracing::info!("finished_bool :{}", b);
        futures::future::ready(Ok((self, s, b)))
    }

    fn finished(self, s: Session) -> Self::FutureUnit {
        tracing::info!("finished");
        futures::future::ready(Ok((self, s)))
    }

    fn channel_open_session(self, channel: ChannelId, session: Session) -> Self::FutureUnit {
        tracing::info!("channel_open_session: {:?} ", channel);
        {
            let mut clients = self.clients.lock().unwrap();
            clients.insert((self.id, channel), session.handle());
            tracing::info!("client map size: {}", clients.len());
        }
        self.finished(session)
    }

    fn exec_request(
        self,
        channel: ChannelId,
        data: &[u8],
        mut session: Session,
    ) -> Self::FutureUnit {
        let data = String::from_utf8_lossy(data).trim().to_owned();
        tracing::info!("exec: {:?},{}", channel, data);
        let res = self.handle_git_command(&data);
        session.data(channel, res.into());
        self.finished(session)
    }

    fn auth_publickey(self, user: &str, public_key: &key::PublicKey) -> Self::FutureAuth {
        tracing::info!("auth_publickey: {} / {:?}", user, public_key);
        self.finished_auth(server::Auth::Accept)
    }

    fn auth_password(self, user: &str, password: &str) -> Self::FutureAuth {
        tracing::info!("auth_password: {} / {}", user, password);
        // in this example implementation, any username/password combination is accepted
        self.finished_auth(server::Auth::Accept)
    }

    fn data(self, channel: ChannelId, data: &[u8], mut session: Session) -> Self::FutureUnit {
        tracing::info!("data: {}", String::from_utf8_lossy(data).trim().to_owned());
        {
            let mut clients = self.clients.lock().unwrap();
            for ((id, channel), ref mut s) in clients.iter_mut() {
                if *id != self.id {
                    s.data(*channel, CryptoVec::from_slice(data));
                }
            }
        }
        session.data(channel, CryptoVec::from_slice(data));
        self.finished(session)
    }
}

impl<T> SshServer<T>
where
    T: ObjectStorage + Send + Sync + Clone,
{
    fn handle_git_command(&self, command: &str) -> String {
        let command: Vec<_> = command.split(' ').collect();
        let mut http_protocol =
            HttpProtocol::new(PathBuf::from(command[1]), command[0], self.storage.clone());
        // tracing::info!("protocol: {:?}", http_protocol);
        let res = "".to_owned();

        // let storage: Box<T> = Box::new(self.storage);
        let handle = self.handle.clone();
        std::thread::spawn(move || {
            let pkt_line_stream = handle.block_on(http_protocol.git_info_refs());
            String::from_utf8(pkt_line_stream.to_vec()).unwrap()
        });
        res
    }
}
