use blkstructs::{Transaction, CoinDataHeight};

use crate::common::context::ExecutionContext;
use crate::wallet::manager::WalletManager;
use crate::wallet::wallet::Wallet;

/// Responsible for common exeuction between wallet_shell and non-wallet_shell modes.
pub struct CommonExecutor {
    pub context: ExecutionContext,
}

impl CommonExecutor {
    pub fn new(context: ExecutionContext) -> Self {
        Self { context }
    }

    /// Creates a new wallet, stores it into db and outputs the name & secret.
    pub async fn create_wallet(&self, wallet_name: &str) -> anyhow::Result<Wallet> {
        let manager = WalletManager::new(self.context.clone());
        let wallet = manager.create_wallet(wallet_name).await?;
        Ok(wallet)
    }

    /// Check transaction until it is confirmed.
    /// TODO: we may need a max timeout to set upper bound on tx polling.
    pub async fn confirm_tx(
        &self,
        tx: &Transaction,
        wallet: &Wallet,
        sleep_sec: u64,
    ) -> anyhow::Result<CoinDataHeight> {
        loop {
            let (coin_data_height, coin_id) = wallet.check_tx(tx).await?;
            if self.context.formatter.is_none() {}
            if let Some(formatter) = self.context.formatter {

            }
            output2::check_coin(&coin_data_height, &coin_id).await;
            self.context.sleep(sleep_sec).await?;
        }
    }

    /// Adds coins by coin id to wallet.
    pub async fn add_coins(
        &self,
        wallet_name: &str,
        secret: &str,
        coin_id: &str,
    ) -> anyhow::Result<()> {
        unimplemented!();
        // Ok(())
    }

    /// Shows the total known wallet balance.
    pub async fn show_balance(&self, wallet_name: &str, secret: &str) -> anyhow::Result<()> {
        unimplemented!();
        // Ok(())
    }

    /// Shows all the wallets by name that are stored in the db.
    pub async fn show_wallets(&self) -> anyhow::Result<()> {
        unimplemented!();
        // Ok(())
    }
}