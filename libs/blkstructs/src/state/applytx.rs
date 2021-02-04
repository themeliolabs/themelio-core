use std::convert::TryInto;

use dashmap::DashMap;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tmelcrypt::HashVal;

use crate::{
    CoinData, CoinDataHeight, CoinID, StakeDoc, State, StateError, Transaction, TxKind,
    COVHASH_ABID, COVHASH_DESTROY, DENOM_DOSC, DENOM_TMEL, DENOM_TSYM, STAKE_EPOCH,
};

use super::melmint;

/// A mutable "handle" to a particular State. Can be "committed" like a database transaction.
/// Note: Option type values are used to indicate deletion when None
pub(crate) struct StateHandle<'a> {
    state: &'a mut State,

    coin_cache: DashMap<CoinID, Option<CoinDataHeight>>,
    transactions_cache: DashMap<HashVal, Transaction>,

    fee_pool_cache: u128,
    tips_cache: u128,

    auction_bids_cache: DashMap<HashVal, Option<Transaction>>,

    stakes_cache: DashMap<HashVal, StakeDoc>,
}

impl<'a> StateHandle<'a> {
    pub fn new(state: &'a mut State) -> Self {
        let fee_pool_cache = state.fee_pool;
        let tips_cache = state.tips;

        StateHandle {
            state,

            coin_cache: DashMap::new(),
            transactions_cache: DashMap::new(),

            fee_pool_cache,
            tips_cache,

            auction_bids_cache: DashMap::new(),

            stakes_cache: DashMap::new(),
        }
    }

    pub fn apply_tx_batch(&mut self, txx: &[Transaction]) -> Result<(), StateError> {
        for tx in txx {
            if !tx.is_well_formed() {
                return Err(StateError::MalformedTx);
            }
            self.transactions_cache.insert(tx.hash_nosigs(), tx.clone());
            self.apply_tx_fees(tx)?;
        }
        // apply outputs in parallel
        txx.par_iter()
            .map(|tx| self.apply_tx_outputs(tx))
            .collect::<Result<_, _>>()?;
        // apply inputs in parallel
        txx.par_iter()
            .map(|tx| self.apply_tx_inputs(tx))
            .collect::<Result<_, _>>()?;
        // apply specials in parallel
        txx.par_iter()
            .filter(|tx| tx.kind != TxKind::Normal && tx.kind != TxKind::Faucet)
            .map(|tx| self.apply_tx_special(tx))
            .collect::<Result<_, _>>()?;
        Ok(())
    }

    pub fn commit(self) {
        // commit coins
        for (k, v) in self.coin_cache {
            if let Some(v) = v {
                self.state.coins.insert(k, v);
            } else {
                self.state.coins.delete(&k);
            }
        }
        // commit txx
        for (k, v) in self.transactions_cache {
            self.state.transactions.insert(k, v);
        }
        // commit fees
        self.state.fee_pool = self.fee_pool_cache;
        self.state.tips = self.tips_cache;
        // commit abids
        for (k, v) in self.auction_bids_cache {
            if let Some(v) = v {
                self.state.auction_bids.insert(k, v);
            } else {
                self.state.auction_bids.delete(&k);
            }
        }
        // commit stakes
        for (k, v) in self.stakes_cache {
            self.state.stakes.insert(k, v);
        }
    }

    fn apply_tx_inputs(&self, tx: &Transaction) -> Result<(), StateError> {
        let scripts = tx.script_as_map();
        // build a map of input coins
        let mut in_coins: im::HashMap<Vec<u8>, u128> = im::HashMap::new();
        // iterate through the inputs
        for coin_id in tx.inputs.iter() {
            if self.get_stake(coin_id.txhash).is_some()
                || (self.get_abid(coin_id.txhash).is_some()
                    && tx.kind != TxKind::AuctionBuyout
                    && tx.kind != TxKind::AuctionFill)
            {
                return Err(StateError::CoinLocked);
            }
            let coin_data = self.get_coin(*coin_id);
            match coin_data {
                None => return Err(StateError::NonexistentCoin(*coin_id)),
                Some(coin_data) => {
                    log::trace!(
                        "coin_data {:?} => {:?} for txid {:?}",
                        coin_id,
                        coin_data,
                        tx.hash_nosigs()
                    );
                    let script = scripts
                        .get(&coin_data.coin_data.covhash)
                        .ok_or(StateError::NonexistentScript(coin_data.coin_data.covhash))?;
                    // we skip checking the script if it's ABID and the tx type is buyout or fill
                    if !(coin_data.coin_data.covhash == *COVHASH_ABID
                        && (tx.kind == TxKind::AuctionBuyout || tx.kind == TxKind::AuctionFill))
                        && !script.check(tx)
                    {
                        return Err(StateError::ViolatesScript(coin_data.coin_data.covhash));
                    }
                    // we need expression to be false
                    // expression has two parts 1 & 2 seperated by an &&
                    //
                    self.del_coin(*coin_id);
                    in_coins.insert(
                        coin_data.coin_data.denom.clone(),
                        in_coins.get(&coin_data.coin_data.denom).unwrap_or(&0)
                            + coin_data.coin_data.value,
                    );
                }
            }
        }
        // balance inputs and outputs. ignore outputs with empty cointype (they create a new token kind)
        let out_coins = tx.total_outputs();
        if tx.kind != TxKind::Faucet {
            for (currency, value) in out_coins.iter() {
                // we skip the created doscs for a DoscMint transaction
                if tx.kind == TxKind::DoscMint && currency == &DENOM_DOSC {
                    continue;
                }
                if !currency.is_empty() && *value != *in_coins.get(currency).unwrap_or(&u128::MAX) {
                    return Err(StateError::UnbalancedInOut);
                }
            }
        }
        Ok(())
    }

    fn apply_tx_fees(&mut self, tx: &Transaction) -> Result<(), StateError> {
        // fees
        let min_fee = self.state.fee_multiplier.saturating_mul(tx.weight(0));
        if tx.fee < min_fee {
            return Err(StateError::InsufficientFees(min_fee));
        }
        let tips = tx.fee - min_fee;
        self.tips_cache = self.tips_cache.saturating_add(tips);
        self.fee_pool_cache = self.fee_pool_cache.saturating_add(min_fee);
        Ok(())
    }

    fn apply_tx_outputs(&self, tx: &Transaction) -> Result<(), StateError> {
        let height = self.state.height;
        for (index, coin_data) in tx.outputs.iter().enumerate() {
            // if covenant hash is zero, this destroys the coins permanently
            if coin_data.covhash != COVHASH_DESTROY {
                self.set_coin(
                    CoinID {
                        txhash: tx.hash_nosigs(),
                        index: index.try_into().unwrap(),
                    },
                    CoinDataHeight {
                        coin_data: coin_data.clone(),
                        height,
                    },
                );
            }
        }
        Ok(())
    }

    fn apply_tx_special(&self, tx: &Transaction) -> Result<(), StateError> {
        match tx.kind {
            TxKind::DoscMint => self.apply_tx_special_doscmint(tx),
            TxKind::AuctionBid => self.apply_tx_special_auctionbid(tx),
            TxKind::AuctionBuyout => self.apply_tx_special_auction_buyout(tx),
            TxKind::AuctionFill => {
                // intentionally ignore here. the auction-fill effects are done elsewhere.
                Ok(())
            }
            TxKind::Stake => self.apply_tx_special_stake(tx),
            _ => panic!("tried to apply special effects of a non-special transaction"),
        }
    }

    fn apply_tx_special_doscmint(&self, tx: &Transaction) -> Result<(), StateError> {
        let coin_id = *tx.inputs.get(0).ok_or(StateError::MalformedTx)?;
        let coin_data = self.get_coin(coin_id).ok_or(StateError::MalformedTx)?;
        // make sure the time is long enough that we can easily measure it
        if self.state.height - coin_data.height < 100 {
            return Err(StateError::InvalidMelPoW);
        }
        // construct puzzle seed
        let chi = tmelcrypt::hash_keyed(
            &self.state.history.get(&coin_data.height).0.unwrap().hash(),
            &stdcode::serialize(tx.inputs.get(0).ok_or(StateError::MalformedTx)?).unwrap(),
        );
        // get difficulty and proof
        let (difficulty, proof): (u32, Vec<u8>) =
            stdcode::deserialize(&tx.data).map_err(|_| StateError::MalformedTx)?;
        let proof = melpow::Proof::from_bytes(&proof).ok_or(StateError::MalformedTx)?;
        if !proof.verify(&chi, difficulty as _) {
            return Err(StateError::InvalidMelPoW);
        }
        // compute speeds
        let my_speed = 2u128.pow(difficulty);
        let reward_real = melmint::calculate_reward(my_speed, self.state.dosc_speed, difficulty);
        let reward_nom = melmint::dosc_inflate_r2n(self.state.height, reward_real);
        // ensure that the total output of DOSCs is correct
        let total_dosc_output = tx
            .total_outputs()
            .get(DENOM_DOSC)
            .cloned()
            .unwrap_or_default();
        if total_dosc_output > reward_nom {
            return Err(StateError::InvalidMelPoW);
        }
        Ok(())
    }

    fn apply_tx_special_auctionbid(&self, tx: &Transaction) -> Result<(), StateError> {
        // must be in first half of auction
        if self.state.height % 20 >= 10 {
            return Err(StateError::BidWrongTime);
        }
        // data must be a 32-byte conshash
        if tx.data.len() != 32 {
            return Err(StateError::MalformedTx);
        }
        // first output stores the price bid for the syms
        let first_output = tx.outputs.get(0).ok_or(StateError::MalformedTx)?;
        if first_output.denom != DENOM_DOSC {
            return Err(StateError::MalformedTx);
        }
        // first output must have a special script
        if first_output.covhash != *COVHASH_ABID {
            return Err(StateError::MalformedTx);
        }
        // save transaction to auction list
        self.set_abid(tx.clone());
        Ok(())
    }

    fn apply_tx_special_auction_buyout(&self, tx: &Transaction) -> Result<(), StateError> {
        let abid_txx: Vec<Transaction> = tx
            .inputs
            .iter()
            .filter_map(|cid| self.get_abid(cid.txhash))
            .collect();
        if abid_txx.len() != 1 {
            return Err(StateError::MalformedTx);
        }
        let abid_txx = &abid_txx[0];
        // validate that the first output fills the order
        let first_output: &CoinData = tx.outputs.get(0).ok_or(StateError::MalformedTx)?;
        if first_output.denom != DENOM_TSYM
            || first_output.value < abid_txx.outputs[0].value
            || first_output.covhash.0.to_vec() != abid_txx.data
        {
            return Err(StateError::MalformedTx);
        }
        // remove the order from the order book
        self.del_abid(abid_txx.hash_nosigs());
        Ok(())
    }

    fn apply_tx_special_stake(&self, tx: &Transaction) -> Result<(), StateError> {
        // first we check that the data is correct
        let stake_doc: StakeDoc =
            stdcode::deserialize(&tx.data).map_err(|_| StateError::MalformedTx)?;
        let curr_epoch = self.state.height / STAKE_EPOCH;
        // then we check that the first coin is valid
        let first_coin = tx.outputs.get(0).ok_or(StateError::MalformedTx)?;
        if first_coin.denom != DENOM_TMEL.to_vec() {
            return Err(StateError::MalformedTx);
        }
        // then we check consistency
        if !(stake_doc.e_start > curr_epoch
            && stake_doc.e_post_end > stake_doc.e_start
            && stake_doc.syms_staked == first_coin.value)
        {
            self.set_stake(tx.hash_nosigs(), stake_doc);
        }
        Ok(())
    }

    fn get_coin(&self, coin_id: CoinID) -> Option<CoinDataHeight> {
        self.coin_cache
            .entry(coin_id)
            .or_insert_with(|| self.state.coins.get(&coin_id).0)
            .value()
            .clone()
    }

    fn set_coin(&self, coin_id: CoinID, value: CoinDataHeight) {
        self.coin_cache.insert(coin_id, Some(value));
    }

    fn del_coin(&self, coin_id: CoinID) {
        self.coin_cache.insert(coin_id, None);
    }

    fn get_abid(&self, txhash: HashVal) -> Option<Transaction> {
        self.auction_bids_cache
            .entry(txhash)
            .or_insert_with(|| self.state.auction_bids.get(&txhash).0)
            .value()
            .clone()
    }

    fn set_abid(&self, tx: Transaction) {
        self.auction_bids_cache.insert(tx.hash_nosigs(), Some(tx));
    }

    fn del_abid(&self, txhash: HashVal) {
        self.auction_bids_cache.insert(txhash, None);
    }

    fn get_stake(&self, txhash: HashVal) -> Option<StakeDoc> {
        if let Some(cached_sd) = self.stakes_cache.get(&txhash).as_deref() {
            return Some(cached_sd).cloned()
        }
        if let Some(sd) = self.state.stakes.get(&txhash).0 {
            return self.stakes_cache.insert(txhash, sd)
        }
        None
    }

    fn set_stake(&self, txhash: HashVal, sdoc: StakeDoc) {
        self.stakes_cache.insert(txhash, sdoc);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use rstest::*;
    use crate::testing::fixtures::*;
    use crate::testing::factory::*;
    use crate::{State, TxKind, CoinID, CoinData};
    use crate::state::applytx::StateHandle;
    use crate::melscript::Script;
    use tmelcrypt::{Ed25519PK, Ed25519SK};

    #[rstest]
    fn test_apply_tx_inputs_single_valid_tx(
        genesis_state: State,
        genesis_mel_coin_id: CoinID,
        genesis_mel_coin_data: CoinData,
        genesis_cov_script_keypair: (Ed25519PK, Ed25519SK),
        genesis_cov_script: Script,
        keypair: (Ed25519PK, Ed25519SK)
    ) {
        // Init state and state handle
        let mut state = genesis_state.clone();
        let state_handle = StateHandle::new(&mut state);

        // Create a valid signed transaction from first coin
        let fee = 3000000;
        let tx = tx_factory(
            TxKind::Normal,
            genesis_cov_script_keypair,
            keypair.0,
            genesis_mel_coin_id,
            genesis_cov_script,
            genesis_mel_coin_data.value,
            fee
        );

        // Apply tx inputs and verify no error
        let res = state_handle.apply_tx_inputs(&tx);

        assert!(res.is_ok());
    }
}