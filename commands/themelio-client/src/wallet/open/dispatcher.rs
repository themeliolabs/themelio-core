use crate::wallet::wallet::Wallet;
use crate::wallet::open::prompter::{Input, Output};
use crate::wallet::open::command::OpenWalletCommand;

pub struct OpenWalletDispatcher {
    host: smol::net::SocketAddr,
    database: std::path::PathBuf,
    version: String,
    name: String,
    secret: String,
    locked: bool,
}

impl OpenWalletDispatcher {
    pub(crate) fn new(host: &smol::net::SocketAddr, database: &std::path::PathBuf, version: &str, name: &str, secret: &str) -> Self {
        let host = host.clone();
        let database = database.clone();
        let version = version.to_string();
        let name = name.to_string();
        let secret = secret.to_string();
        let locked = true;
        Self { host, database, version, name, secret, locked }
    }

    /// Dispatch commands from user input and show output using prompt until user exits.
    pub(crate) async fn run(&self) -> anyhow::Result<()> {
        // Ensure we can unlock the wallet given the wallet name and secret.
        let valid_secret = Wallet::check_secret(&self.host, &self.database, &self.name, &self.secret).await?;
        if !valid_secret {
            anyhow::bail!(ClientError::InvalidWalletSecret(name));
        }

        // Format user prompt.
        let prompt = Input::format_prompt(&self.version, &self.name).await?;

        loop {
            // Get command from user input.
            let open_cmd = Input::command(&prompt).await?;

            // Exit if the user chooses to exit.
            if open_cmd == OpenWalletCommand::Exit {
                Output::exit().await?;
                return Ok(());
            }

            // Dispatch the command.
            let dispatch_result = &self.dispatch(&open_cmd, &wallet).await;

            // Output error, if any, and continue running.
            match dispatch_result {
                Err(err) => Output::error(err, &open_cmd).await?,
                _ => {}
            }
        }
    }

    /// Dispatch and process the command.
    pub(crate) async fn dispatch(&self, open_cmd: &OpenWalletCommand, wallet: &Wallet) -> anyhow::Result<()> {
        // Dispatch a command and return a command result
        match &open_cmd {
            OpenWalletCommand::Faucet(amt, denom) => { self.faucet(amt, denom, wallet).await?; }
            OpenWalletCommand::Deposit => { todo!("") }
            OpenWalletCommand::Withdraw => { todo!("") }
            OpenWalletCommand::Swap => { todo!("") }
            OpenWalletCommand::SendCoins(dest, amt, denom) => { self.send_coins(dest, amt, denom, wallet).await?; }
            OpenWalletCommand::AddCoins(coin_id) => { self.add_coins(coin_id, wallet).await?; }
            OpenWalletCommand::Balance => { self.balance().await?; }
            OpenWalletCommand::Help => { self.help().await?; }
            OpenWalletCommand::Exit => {}
        }
        Ok(())
    }

    async fn faucet(&self, amt: &str, denom: &str) -> anyhow::Result<()> {
        let wallet = Wallet::new(&self.host, &self.database);

        let wallet_data = wallet.open(&self.name, &self.secret).await?;

        let coin = wallet.faucet(&wallet_data, self.amt, self.denom).await?;

        prompter::output_faucet_tx(wallet_data, coin).await?;

        self.confirm_faucet_tx(coin).await?;

        prompter::faucet_tx_confirmed().await?;

        Ok(())
    }

    async fn confirm_faucet(&self, coin_id: CoinId) -> anyhow::Result<()> {
        loop {

            prompter::faucet_tx_confirming().await?;
        }
        //                 eprintln!(
//                     ">> Faucet transaction for {} mels broadcast!",
//                     number.to_string().bold()
//                 );
//                 eprintln!(">> Waiting for confirmation...");
//                 // loop until we get coin data height and proof from last header
//                 loop {
//                     let (coin_data_height, _hdr) = active_wallet.get_coin_data(coin).await?;
//                     if let Some(cd_height) = coin_data_height {
//                         eprintln!(
//                             ">>> Coin is confirmed at current height {}",
//                             cd_height.height
//                         );

//                         eprintln!(
//                             ">> CID = {}",
//                             hex::encode(stdcode::serialize(&coin).unwrap()).bold()
//                         );
//                         break;
//                     }
    }
    async fn send_coins(&self, amt: &str, denom: &str) -> anyhow::Result<()> {
        let wallet = Wallet::new(&self.host, &self.database);
        let wallet_data = wallet.open(&self.name, &self.secret).await?;
        // let prompt = open::prompt::format_prompt(&self.version).await?;
        //                 let tx = active_wallet.create_tx(dest_addr, amount, unit).await?;
//                 let fee_prompt = format!("Do you wish to send a tx with a fee of {} (y/n): ", tx.fee);
//                 let fee_input = read_line(fee_prompt.to_string()).await.unwrap();
//                 if !fee_input.contains('y') {
//                     continue;
//                 }

//                 let tx = active_wallet.send_tx(tx).await?;
//                 eprintln!(">> Sent tx.  Waiting to verify.");
//                 loop {
//                     let (coin_data_height, _proof) = active_wallet.verify_tx(tx.clone()).await?;
//                     if let Some(out) = coin_data_height {
//                         let their_coin = CoinID {
//                             txhash: tx.hash_nosigs(),
//                             index: 0,
//                         };
//                         let first_change = CoinID {
//                             txhash: tx.hash_nosigs(),
//                             index: 1,
//                         };
//                         eprintln!(">> Confirmed at height {}!", out.height);
//                         eprintln!(
//                             ">> CID (Sent) = {}",
//                             hex::encode(stdcode::serialize(&their_coin).unwrap()).bold()
//                         );
//                         eprintln!(
//                             ">> CID (Change) = {}",
//                             hex::encode(stdcode::serialize(&first_change).unwrap()).bold()
//                         );
//                         break;
//                     }
//                 }
    }
    async fn add_coins(&self, amt: &str, denom: &str) -> anyhow::Result<()> {
        let wallet = Wallet::new(&self.host, &self.database);
        let wallet_data = wallet.open(&self.name, &self.secret).await?;
        // let prompt = open::prompt::format_prompt(&self.version).await?;
        //                 let (coin_data_height, coin_id, _full_proof) =
//                     active_wallet.get_coin_data_by_id(coin_id).await?;
//                 match coin_data_height {
//                     None => {
//                         eprintln!("Coin not found");
//                         continue;
//                     }
//                     Some(coin_data_height) => {
//                         eprintln!(
//                             ">> Coin found at height {}! Added {} {} to data",
//                             coin_data_height.height,
//                             coin_data_height.coin_data.value,
//                             {
//                                 let val = coin_data_height.coin_data.denom.as_slice();
//                                 format!("X-{}", hex::encode(val))
//                             }
//                         );
//                         active_wallet.add_coin(&coin_id, &coin_data_height).await?;
//                         eprintln!("Added coin to wallet");
//                     }
//                 }
    }
    async fn balance(&self) -> anyhow::Result<()> {
        let wallet = Wallet::new(&self.host, &self.database);
        let wallet_data = wallet.open(&self.name, &self.secret).await?;
        // let prompt = open::prompt::format_prompt(&self.version).await?;
        //                 let balance = active_wallet.get_balance().await?;
//                 eprintln!(">> **** BALANCE ****");
//                 eprintln!(">> {}", balance);
    }

    /// Show available open wallet inputs to user
    async fn help(&self) -> anyhow::Result<()> {
        prompter::output_help().await?;
        Ok(())
    }
}