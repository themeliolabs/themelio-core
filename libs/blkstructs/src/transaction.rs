use crate::constants::*;
use crate::melscript::*;
use arbitrary::Arbitrary;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(
    Clone,
    Copy,
    IntoPrimitive,
    TryFromPrimitive,
    Eq,
    PartialEq,
    Arbitrary,
    Debug,
    Serialize,
    Deserialize,
)]
#[repr(u8)]
pub enum TxKind {
    Normal = 0x00,
    Stake = 0x10,
    DoscMint = 0x50,
    AuctionBid = 0x51,
    AuctionBuyout = 0x52,
    AuctionFill = 0x53,

    Faucet = 0xff,
}

/// Transaction represents an individual, RLP-serializable Themelio transaction.
#[derive(Clone, Arbitrary, Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub kind: TxKind,
    pub inputs: Vec<CoinID>,
    pub outputs: Vec<CoinData>,
    pub fee: u64,
    pub scripts: Vec<Script>,
    pub data: Vec<u8>,
    pub sigs: Vec<Vecu8>,
}

type Vecu8 = Vec<u8>;

impl Transaction {
    pub fn empty_test() -> Self {
        Transaction {
            kind: TxKind::Normal,
            inputs: Vec::new(),
            outputs: Vec::new(),
            fee: 0,
            scripts: Vec::new(),
            data: Vec::new(),
            sigs: Vec::new(),
        }
    }
    /// checks whether or not the transaction is well formed, respecting coin size bounds and such.
    pub fn is_well_formed(&self) -> bool {
        // check bounds
        for out in self.outputs.iter() {
            if out.value > MAX_COINVAL {
                return false;
            }
        }
        if self.fee > MAX_COINVAL {
            return false;
        }
        if self.outputs.len() > 255 || self.inputs.len() > 255 {
            return false;
        }
        true
    }
    /// hash_nosigs returns the hash of the transaction with a zeroed-out signature field. This is what signatures are computed against.
    pub fn hash_nosigs(&self) -> tmelcrypt::HashVal {
        let mut s = self.clone();
        s.sigs = vec![];
        let self_bytes = bincode::serialize(&s).unwrap();
        tmelcrypt::hash_single(&self_bytes)
    }
    /// sign_ed25519 appends an ed25519 signature to the transaction.
    pub fn sign_ed25519(mut self, sk: tmelcrypt::Ed25519SK) -> Self {
        self.sigs.push(sk.sign(&self.hash_nosigs().0));
        self
    }
    /// total_outputs returns a HashMap mapping each type of coin to its total value. Fees will be included in COINTYPE_TMEL.
    pub fn total_outputs(&self) -> HashMap<Vec<u8>, u64> {
        let mut toret = HashMap::new();
        for output in self.outputs.iter() {
            let old = *toret.get(&output.cointype).unwrap_or(&0);
            toret.insert(output.cointype.clone(), old + output.value);
        }
        let old = *toret.get(COINTYPE_TMEL).unwrap_or(&0);
        toret.insert(COINTYPE_TMEL.to_vec(), old + self.fee);
        toret
    }
    /// scripts_as_map returns a HashMap mapping the hash of each script in the transaction to the script itself.
    pub fn script_as_map(&self) -> HashMap<tmelcrypt::HashVal, Script> {
        let mut toret = HashMap::new();
        for s in self.scripts.iter() {
            toret.insert(s.hash(), s.clone());
        }
        toret
    }
}

#[derive(
    Serialize, Deserialize, Clone, Debug, Copy, Arbitrary, Ord, PartialOrd, Eq, PartialEq, Hash,
)]
pub struct CoinID {
    pub txhash: tmelcrypt::HashVal,
    pub index: u8,
}

#[derive(Serialize, Deserialize, Clone, Arbitrary, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct CoinData {
    pub conshash: tmelcrypt::HashVal,
    pub value: u64,
    pub cointype: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Arbitrary, Debug)]
pub struct CoinDataHeight {
    pub coin_data: CoinData,
    pub height: u64,
}
