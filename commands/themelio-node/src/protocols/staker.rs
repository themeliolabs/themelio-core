use crate::services::storage::SharedStorage;

use themelio_stf::{melvm::Address, Block, ProposerAction, SealedState, STAKE_EPOCH};

use novasymph::BlockBuilder;
use smol::prelude::*;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tmelcrypt::Ed25519SK;
use tracing::instrument;

/// This encapsulates the staker-specific peer-to-peer.
pub struct StakerProtocol {
    _network_task: smol::Task<()>,
}

impl StakerProtocol {
    /// Creates a new instance of the staker protocol.
    pub fn new(
        addr: SocketAddr,
        bootstrap: Vec<SocketAddr>,
        storage: SharedStorage,
        my_sk: Ed25519SK,
        payout_covhash: Address,
        target_fee_multiplier: u128,
    ) -> anyhow::Result<Self> {
        let _network_task = smolscale::spawn(async move {
            loop {
                let x = storage.read().highest_height();
                smol::Timer::after(Duration::from_secs(60)).await;
                let y = storage.read().highest_height();
                log::info!(
                    "delta-height = {}; must be less than 5 to start staker",
                    y - x
                );
                if y - x < 5 {
                    break;
                }
            }
            loop {
                let genesis_epoch = storage.read().highest_height() / STAKE_EPOCH;
                for current_epoch in genesis_epoch.. {
                    log::info!("epoch transitioning into {}!", current_epoch);
                    // we race the staker loop with epoch termination. epoch termination for now is just a sleep loop that waits until the last block in the epoch is confirmed.
                    let staker_fut = one_epoch_loop(
                        current_epoch,
                        addr,
                        bootstrap.clone(),
                        storage.clone(),
                        my_sk,
                        payout_covhash,
                        target_fee_multiplier,
                    );
                    let epoch_termination = async {
                        loop {
                            smol::Timer::after(Duration::from_secs(1)).await;
                            if (storage.read().highest_height() + 1) / STAKE_EPOCH != current_epoch
                            {
                                break Ok(());
                            }
                        }
                    };
                    if let Err(err) = staker_fut.race(epoch_termination).await {
                        log::warn!("staker rebooting: {:?}", err);
                        break;
                    }
                }
            }
        });
        Ok(Self { _network_task })
    }
}

#[allow(clippy::or_fun_call)]
#[instrument(skip(storage, my_sk))]
async fn one_epoch_loop(
    epoch: u64,
    addr: SocketAddr,
    bootstrap: Vec<SocketAddr>,
    storage: SharedStorage,
    my_sk: Ed25519SK,
    payout_covhash: Address,
    target_fee_multiplier: u128,
) -> anyhow::Result<()> {
    let genesis = storage.read().highest_state();
    let forest = storage.clone().read().forest();
    let config = novasymph::EpochConfig {
        listen: addr,
        bootstrap,
        genesis,
        forest,
        start_time: std::time::UNIX_EPOCH + Duration::from_secs(1619758800), // Apr 30 2021
        interval: Duration::from_secs(30),
        signing_sk: my_sk,
        builder: StorageBlockBuilder {
            storage: storage.clone(),
            payout_covhash,
            target_fee_multiplier,
        },
        get_confirmed: {
            let storage = storage.clone();
            Box::new(move |height: u64| {
                let storage = storage.read();
                storage
                    .get_state(height)?
                    .confirm(storage.get_consensus(height)?, None)
            })
        },
    };
    let protocol = Arc::new(novasymph::EpochProtocol::new(config));
    let main_loop = async {
        loop {
            let confirmed = protocol.next_confirmed().await;
            let height = confirmed.inner().inner_ref().height;
            let mut storage = storage.write();
            if let Err(err) =
                storage.apply_block(confirmed.inner().to_block(), confirmed.cproof().clone())
            {
                log::warn!(
                    "could not apply confirmed block {} from novasymph: {:?}",
                    height,
                    err
                )
            }
        }
    };
    let reset_loop = async {
        loop {
            let latest_known = storage.read().highest_state();
            let protocol = protocol.clone();
            smol::unblock(move || protocol.reset_genesis(latest_known)).await;
            smol::Timer::after(Duration::from_secs(5)).await;
        }
    };
    main_loop.race(reset_loop).await
}

struct StorageBlockBuilder {
    storage: SharedStorage,
    payout_covhash: Address,
    target_fee_multiplier: u128,
}

impl BlockBuilder for StorageBlockBuilder {
    fn build_block(&self, tip: SealedState) -> Block {
        let mut storage = self.storage.write();
        let proposer_action = ProposerAction {
            fee_multiplier_delta: if tip.header().fee_multiplier > self.target_fee_multiplier {
                i8::MIN
            } else {
                i8::MAX
            },
            reward_dest: self.payout_covhash,
        };
        let mempool_state = storage.mempool().to_state().seal(Some(proposer_action));
        if mempool_state.header().previous != tip.header().hash() {
            log::warn!(
                "mempool {} doesn't extend from tip {}; building quasiempty block",
                mempool_state.header().height,
                tip.header().height
            );
            let next = tip.next_state().seal(Some(proposer_action));
            next.to_block()
        } else {
            storage.mempool_mut().rebase(mempool_state.next_state());
            mempool_state.to_block()
        }
    }

    fn hint_next_build(&self, tip: SealedState) {
        self.storage.write().mempool_mut().rebase(tip.next_state());
    }
}
