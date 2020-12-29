use blkstructs::{melscript, CoinData, CoinDataHeight, CoinID, Transaction, TxKind};
use serde::{Deserialize, Serialize};
use std::collections;
use tmelcrypt::HashVal;

#[derive(Serialize, Deserialize, Debug, Clone)]
/// An immutable, cloneable in-memory wallet that can be synced to disk. Does not contain any secrets!
pub struct Wallet {
    unspent_coins: im::HashMap<CoinID, CoinDataHeight>,
    spent_coins: im::HashMap<CoinID, CoinDataHeight>,
    tx_in_progress: im::HashMap<HashVal, Transaction>,
    pub my_script: melscript::Script,
}

impl Wallet {
    /// Coins
    pub fn unspent_coins(&self) -> impl Iterator<Item = (&CoinID, &CoinDataHeight)> {
        self.unspent_coins.iter()
    }
    /// Create a new wallet.
    pub fn new(my_script: melscript::Script) -> Self {
        Wallet {
            unspent_coins: im::HashMap::new(),
            spent_coins: im::HashMap::new(),
            tx_in_progress: im::HashMap::new(),
            my_script,
        }
    }
    /// Inserts a coin into the wallet, returning whether or not the coin already exists.
    pub fn insert_coin(&mut self, coin_id: CoinID, coin_data_height: CoinDataHeight) -> bool {
        self.spent_coins.get(&coin_id).is_none()
            && self
                .unspent_coins
                .insert(coin_id, coin_data_height)
                .is_none()
    }

    /// Creates an **unsigned** transaction out of the coins in the wallet. Does not spend it yet.
    pub fn pre_spend(&self, outputs: Vec<CoinData>) -> anyhow::Result<Transaction> {
        // find coins that might match
        let mut txn = Transaction {
            kind: TxKind::Normal,
            inputs: vec![],
            outputs,
            fee: 0,
            scripts: vec![self.my_script.clone()],
            data: vec![],
            sigs: vec![],
        };
        let output_sum = txn.total_outputs();
        let mut input_sum: collections::HashMap<Vec<u8>, u64> = collections::HashMap::new();
        for (coin, data) in self.unspent_coins.iter() {
            let existing_val = input_sum
                .get(&data.coin_data.cointype)
                .cloned()
                .unwrap_or(0);
            if existing_val
                < output_sum
                    .get(&data.coin_data.cointype)
                    .cloned()
                    .unwrap_or(0)
            {
                txn.inputs.push(*coin);
                input_sum.insert(
                    data.coin_data.cointype.clone(),
                    existing_val + data.coin_data.value,
                );
            }
        }
        // create change outputs
        let change = {
            let mut change = Vec::new();
            for (cointype, sum) in output_sum.iter() {
                let difference = input_sum
                    .get(cointype)
                    .unwrap_or(&0)
                    .checked_sub(*sum)
                    .ok_or_else(|| anyhow::anyhow!("not enough money"))?;
                if difference > 0 {
                    change.push(CoinData {
                        conshash: self.my_script.hash(),
                        value: difference,
                        cointype: cointype.clone(),
                    })
                }
            }
            change
        };
        txn.outputs.extend(change.into_iter());
        assert!(txn.is_well_formed());
        Ok(txn)
    }

    /// Actually spend a transaction.
    pub fn spend(&mut self, txn: Transaction) -> anyhow::Result<()> {
        let mut oself = self.clone();
        // move coins from spent to unspent
        for input in txn.inputs.iter().cloned() {
            let coindata = oself
                .unspent_coins
                .remove(&input)
                .ok_or_else(|| anyhow::anyhow!("no such coin in wallet"))?;
            oself.spent_coins.insert(input, coindata);
        }
        // put tx in progress
        self.tx_in_progress.insert(txn.hash_nosigs(), txn);
        // "commit"
        *self = oself;
        Ok(())
    }
}
