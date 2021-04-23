use std::collections::BTreeMap;

use blkstructs::{CoinDataHeight, CoinID};

use crate::utils::formatter::formatter::OutputFormatter;
use crate::wallet::data::WalletData;
use crate::wallet::wallet::Wallet;
use async_trait::async_trait;
use serde::Serialize;

pub struct JsonOutputFormatter {}

#[async_trait]
impl OutputFormatter for JsonOutputFormatter {
    /// Display json of name, secret key and covenant of the wallet.
    async fn wallet(&self, wallet: Wallet) -> anyhow::Result<()> {
        // let json = serde_json::to_string_pretty(&wallet).unwrap();
        // eprintln!("{}", json);
        // Ok(())
        todo!()
    }

    /// Display json of all stored wallet wallet addresses by name.
    async fn wallet_addresses_by_name(
        &self,
        wallets: BTreeMap<String, WalletData>,
    ) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(&wallets).unwrap();
        eprintln!("{}", json);
        Ok(())
    }

    /// Display json showing height and coin id information upon a coin being confimed.
    async fn coin_confirmed(
        &self,
        coin_data_height: &CoinDataHeight,
        coin: &CoinID,
    ) -> anyhow::Result<()> {
        let coin_data_height = coin_data_height.clone();
        let coin = coin.clone();

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Confirmed {
            coin_data_height: CoinDataHeight,
            coin: CoinID,
        }

        let confirmed = Confirmed {
            coin_data_height,
            coin,
        };

        let json = serde_json::to_string_pretty(&confirmed).unwrap();

        eprintln!("{}", json);
        Ok(())
    }

    /// Display message that coin is not yet confirmed.
    async fn coin_pending(&self) -> anyhow::Result<()> {
        let pending_message = "Coin is not yet confirmed".to_string();

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Pending {
            pending_message: String,
        }
        let pending = Pending { pending_message };
        let json = serde_json::to_string_pretty(&pending).unwrap();

        eprintln!("{}", json);
        Ok(())
    }

    /// Display function which displays pending message until a coin is confirmed
    /// at which a confirmed message will be displayed.
    async fn check_coin(
        &self,
        coin_data_height: &Option<CoinDataHeight>,
        coin_id: &CoinID,
    ) -> anyhow::Result<()> {
        match coin_data_height {
            None => self.coin_pending().await?,
            Some(coin_data_height) => self.coin_confirmed(&coin_data_height, &coin_id).await?,
        }
        Ok(())
    }
}
