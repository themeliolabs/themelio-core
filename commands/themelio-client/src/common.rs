use thiserror::Error;
use nodeprot::{ValClientSnapshot, ValClient};

#[derive(Error, Debug)]
/// A error that happens on the client side.
pub enum ClientError {
    #[error("invalid wallet name {:?}", .0)]
    InvalidWalletName(String),
    #[error("wallet with name {:?} already exists", .0)]
    DuplicateWalletName(String),
    #[error("provided secret does not unlock wallet with name {:?} ", .0)]
    InvalidWalletSecret(String),
    #[error("provided invalid input arguments to client {:?} ", .0)]
    InvalidInputArgs(String),
}

/// Contains data for the entire life-cycle of a command being executed.
#[derive(Clone, Debug)]
pub struct ExecutionContext {
    pub host: smol::net::SocketAddr,
    pub network: blkstructs::NetID,
    pub database: std::path::PathBuf,
    pub version: String,
}

impl ExecutionContext {
    /// Get the latest snapshot from an execution context.
    pub async fn get_latest_snapshot(&self) -> anyhow::Result<ValClientSnapshot>{
        let client = ValClient::new(self.network, self.host);
        let snapshot = client.snapshot_latest().await?;
        Ok(snapshot)
    }
}

/// Handle raw user input using a prompt.
pub async fn read_line(prompt: String) -> anyhow::Result<String> {
    smol::unblock(move || {
        let mut rl = rustyline::Editor::<()>::new();
        Ok(rl.readline(&prompt)?)
    })
    .await
}
