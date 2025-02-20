//! RPC types that are supported by Beaverbuild
use alloy_primitives::{Address, BlockNumber, TxHash};
use reth_primitives::TransactionSigned;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};

/// Bundle as recognised by Beaverbuild
///
/// Consult <https://beaverbuild.org/docs.html>. Note that the deprecated `replacementUuid` field
/// has been omitted.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde_as]
pub struct BeaverBundle {
    /// List of hex-encoded, raw transactions. Can be empty for cancelling a bundle
    #[serde(rename = "txs")]
    pub transactions: Vec<TransactionSigned>,
    /// The block that this bundle will be valid for. 0 means it's valid for the next block (and only this one)
    #[serde(with = "alloy_serde::quantity")]
    pub block_number: BlockNumber,
    /// If specified and >0, the bundle will only be valid if the block timestamp is greater or equal to `minTimestamp`
    pub min_timestamp: Option<u64>,
    /// If specified and >0, the bundle will only be valid if the block timestamp is smaller or equal to `maxTimestamp`
    pub max_timestamp: Option<u64>,
    /// A list of transaction hashes contained in the bundle, that can be allowed to revert, or be removed from your bundle if it's deemed useful
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reverting_transaction_hashes: Vec<TxHash>,
    /// A list of transaction hashes contained in the bundle, that can be allowed to be removed from your bundle if it's deemed useful (but not revert)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dropping_transaction_hashes: Vec<TxHash>,
    /// An UUID string, which allows you to update/cancel your bundles: if you specify an uuid and we already have a bundle with an identical one, we'll forget about the old bundle. So we can only have a single bundle with a certain `uuid` at all times (and we keep the most recent)
    pub uuid: Option<String>,
    /// An integer between 1-99. How much of the total priority fee + coinbase payment you want to be refunded for. This will negatively impact your prioritization because this refund is gonna eat into your bundle payment. Example: if a bundle pays 0.2 ETH of priority fee plus 1 ETH to coinbase, a refundPercent set to 50 will result in a transaction being appended after the bundle, paying 0.59 ETH back to the EOA. This is assuming the payout tx will cost beaver 0.01 ETH in fees, which are deduced from the 0.6 ETH payout.
    pub refund_percent: Option<u64>,
    /// You can specify an address that the funds from `refundPercent` will be sent to. If not specified, they will be sent to the `from` address of the first transaction
    pub refund_recipient: Option<Address>,
    /// The hashes of transactions in the bundle that the refund will be based on. If it's empty, we'll use the last transaction
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub refund_transaction_hashes: Vec<TxHash>,
}
