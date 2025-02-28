use alloy_primitives::{BlockNumber, Bytes, TxHash};
use alloy_rlp::encode;
use serde::Serialize;

pub mod beaver;
pub mod flashbots;
pub mod titan;

pub use beaver::*;
pub use flashbots::*;
pub use titan::*;

/// Universal bundle submission RPC type
///
/// This type represents what Lynx accepts from external order flow providers.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum SendBundleRequest {
    /// Flashbots bundle
    Flashbots(FlashbotsBundle),
    /// Beaverbuild bundle
    Beaver(BeaverBundle),
    /// Titan Builder bundle
    Titan(TitanBundle),
}

impl SendBundleRequest {
    pub fn min_timestamp(&self) -> Option<u64> {
        match self {
            SendBundleRequest::Flashbots(bundle) => bundle.min_timestamp,
            SendBundleRequest::Beaver(bundle) => bundle.min_timestamp,
            SendBundleRequest::Titan(bundle) => bundle.min_timestamp,
        }
    }

    pub fn max_timestamp(&self) -> Option<u64> {
        match self {
            SendBundleRequest::Flashbots(bundle) => bundle.max_timestamp,
            SendBundleRequest::Beaver(bundle) => bundle.max_timestamp,
            SendBundleRequest::Titan(bundle) => bundle.max_timestamp,
        }
    }

    pub fn block_number(&self) -> BlockNumber {
        match self {
            SendBundleRequest::Flashbots(bundle) => bundle.block_number,
            SendBundleRequest::Beaver(bundle) => bundle.block_number,
            SendBundleRequest::Titan(bundle) => bundle.block_number,
        }
    }

    pub fn tx_bytes(&self) -> Vec<Bytes> {
        match self {
            SendBundleRequest::Flashbots(bundle) => bundle.txs.clone(),
            SendBundleRequest::Beaver(bundle) => bundle
                .transactions
                .iter()
                .map(|tx| encode(tx).into())
                .collect(),
            SendBundleRequest::Titan(bundle) => bundle
                .transactions
                .iter()
                .map(|tx| encode(tx).into())
                .collect(),
        }
    }

    pub fn reverting_tx_hashes(&self) -> Vec<TxHash> {
        match self {
            SendBundleRequest::Flashbots(_) => vec![],
            SendBundleRequest::Beaver(bundle) => bundle.reverting_transaction_hashes.clone(),
            SendBundleRequest::Titan(bundle) => bundle.reverting_transaction_hashes.clone(),
        }
    }
}
