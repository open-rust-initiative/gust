//!
//!
//!
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;

use russh_keys::key::KeyPair;

use tokio::io::AsyncWriteExt;

use crate::git::protocol::ssh::SshServer;
use crate::gust::driver::database::mysql;

/// start a ssh server
pub async fn server() -> Result<(), std::io::Error> {
    let client_key = load_key().await.unwrap();
    let client_pubkey = Arc::new(client_key.clone_public_key().unwrap());

    let mut config = russh::server::Config::default();
    config.connection_timeout = Some(std::time::Duration::from_secs(10));
    config.auth_rejection_time = std::time::Duration::from_secs(3);
    config.keys.push(client_key);

    let config = Arc::new(config);
    let sh = SshServer {
        client_pubkey,
        clients: Arc::new(Mutex::new(HashMap::new())),
        id: 0,
        storage: mysql::init().await,
        pack_protocol: None,
    };
    russh::server::run(config, "0.0.0.0:2222", sh).await
}

async fn load_key() -> Result<KeyPair> {
    let key_root = env::var("SSH_ROOT").expect("WORK_DIR is not set in .env file");
    let key_path = PathBuf::from(key_root).join("id_rsa");
    if !key_path.exists() {
        // generate a keypair if not exists
        let keys = KeyPair::generate_ed25519().unwrap();
        let mut key_file = tokio::fs::File::create(&key_path).await.unwrap();

        let KeyPair::Ed25519(inner_pair) = keys;

        key_file.write_all(&inner_pair.to_bytes()).await?;

        Ok(KeyPair::Ed25519(inner_pair))
    } else {
        // load the keypair from the file
        let key_data = tokio::fs::read(&key_path).await?;
        let keypair = ed25519_dalek::Keypair::from_bytes(&key_data)?;

        Ok(KeyPair::Ed25519(keypair))
    }
}
