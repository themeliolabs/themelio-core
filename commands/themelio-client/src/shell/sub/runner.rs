use crate::wallet::manager::WalletManager;
use crate::shell::sub::io::{SubShellInput, SubShellOutput};
use crate::shell::sub::command::SubShellCommand;
use blkstructs::CoinID;
use crate::common::ExecutionContext;

/// A sub-shell runner executed within the higher-level shell.
/// This shell unlocks a wallet, transacts with the network and shows balances.
pub(crate) struct SubShellRunner {
    context: ExecutionContext,
    name: String,
    secret: String,
}

impl SubShellRunner {
    /// Create a new sub shell runner if wallet exists and we can unlock & load with the provided secret.
    pub(crate) async fn new(context: ExecutionContext, name: &str, secret: &str) -> anyhow::Result<Self> {
        let name = name.to_string();
        let secret = secret.to_string();
        let context = context.clone();

        let manager = WalletManager::new(context.clone());
        let _ = manager.load_wallet( &name, &secret).await?;

        Ok(Self { context, name, secret })
    }

    /// Read and execute sub-shell commands from user until user exits.
    pub(crate) async fn run(&self) -> anyhow::Result<()> {
        // Format user prompt.
        let prompt = SubShellInput::format_prompt(&self.context.version, &self.name).await?;

        loop {
            // Get command from user input.
            match SubShellInput::command(&prompt).await {
                Ok(open_cmd) => {
                    // Exit if the user chooses to exit.
                    if open_cmd == SubShellCommand::Exit {
                        SubShellOutput::exit().await?;
                        return Ok(());
                    }

                    // Dispatch the command.
                    let dispatch_result = &self.dispatch(&open_cmd).await;

                    // Output error, if any, and continue running.
                    match dispatch_result {
                        Err(err) => SubShellOutput::subshell_error(err, &open_cmd).await?,
                        _ => {}
                    }
                }
                Err(err) => {
                    SubShellOutput::readline_error(&err).await?
                }
            }


        }
    }

    /// Dispatch and process a single sub-shell command.
    async fn dispatch(&self, sub_shell_cmd: &SubShellCommand) -> anyhow::Result<()> {
        // Dispatch a command and return a command result
        match &sub_shell_cmd {
            SubShellCommand::Faucet(amt, unit) => { self.faucet(amt, unit).await?; }
            SubShellCommand::SendCoins(dest, amt, unit) => { self.send_coins(dest, amt, unit).await?; }
            SubShellCommand::AddCoins(coin_id) => { self.add_coins(coin_id).await?; }
            SubShellCommand::ShowBalance => { self.balance().await?; }
            SubShellCommand::Help => {}
            SubShellCommand::Exit => {}
            // SubShellCommand::Deposit => { todo!("") }
            // SubShellCommand::Withdraw => { todo!("") }
            // SubShellCommand::Swap => { todo!("") }
        }
        Ok(())
    }

    async fn faucet(&self, amt: &str, denom: &str) -> anyhow::Result<()> {
        // let shell = Wallet::new(&self.host, &self.database);
        //
        // let wallet_data = shell.sub(&self.name, &self.secret).await?;
        //
        // let coin = shell.faucet(&wallet_data, self.amt, self.denom).await?;
        //
        // prompter::output_faucet_tx(wallet_data, coin).await?;
        //
        // self.confirm_faucet_tx(coin).await?;
        //
        // prompter::faucet_tx_confirmed().await?;

        Ok(())
    }

    async fn confirm_faucet(&self, _coin_id: CoinID) -> anyhow::Result<()> {
        // loop {
        //
        //     prompter::faucet_tx_confirming().await?;
        // }
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
        Ok(())
    }
    async fn send_coins(&self, dest: &str, amt: &str, unit: &str) -> anyhow::Result<()> {
        // let shell = Wallet::new(&self.host, &self.database);
        // let wallet_data = shell.sub(&self.name, &self.secret).await?;
        // let prompt = sub::prompt::format_prompt(&self.version).await?;
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
        Ok(())
    }
    async fn add_coins(&self, coin_id: &str) -> anyhow::Result<()> {
        // let shell = Wallet::new(&self.host, &self.database);
        // let wallet_data = shell.sub(&self.name, &self.secret).await?;
        // let prompt = sub::prompt::format_prompt(&self.version).await?;
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
//                         eprintln!("Added coin to shell");
//                     }
//                 }
        Ok(())
    }
    async fn balance(&self) -> anyhow::Result<()> {
        Ok(())
        // let shell = Wallet::new(&self.host, &self.database);
        // let wallet_data = shell.sub(&self.name, &self.secret).await?;
        // let prompt = sub::prompt::format_prompt(&self.version).await?;
        //                 let balance = active_wallet.get_balance().await?;
//                 eprintln!(">> **** BALANCE ****");
//                 eprintln!(">> {}", balance);
    }

    /// Show available sub shell inputs to user
    async fn help(&self) -> anyhow::Result<()> {
        // prompter::output_help().await?;
        Ok(())
    }
}