//! RPC types that are supported by Beaverbuild
use alloy_primitives::{hex, Address, BlockNumber, TxHash};
use alloy_rlp::Decodable;
use eyre::eyre;
use reth_primitives::TransactionSigned;
use serde::ser::{Serialize, SerializeStruct, Serializer};

/// Bundle as recognised by Beaverbuild
///
/// Consult <https://beaverbuild.org/docs.html>. Note that the deprecated `replacementUuid` field
/// has been omitted.
#[derive(Clone, Debug, Default)]
pub struct BeaverBundle {
    /// List of hex-encoded, raw transactions. Can be empty for cancelling a bundle
    pub transactions: Vec<TransactionSigned>,
    /// The block that this bundle will be valid for. 0 means it's valid for the next block (and only this one)
    pub block_number: BlockNumber,
    /// If specified and >0, the bundle will only be valid if the block timestamp is greater or equal to `minTimestamp`
    pub min_timestamp: Option<u64>,
    /// If specified and >0, the bundle will only be valid if the block timestamp is smaller or equal to `maxTimestamp`
    pub max_timestamp: Option<u64>,
    /// A list of transaction hashes contained in the bundle, that can be allowed to revert, or be removed from your bundle if it's deemed useful
    pub reverting_transaction_hashes: Vec<TxHash>,
    /// A list of transaction hashes contained in the bundle, that can be allowed to be removed from your bundle if it's deemed useful (but not revert)
    pub dropping_transaction_hashes: Vec<TxHash>,
    /// An UUID string, which allows you to update/cancel your bundles: if you specify an uuid and we already have a bundle with an identical one, we'll forget about the old bundle. So we can only have a single bundle with a certain `uuid` at all times (and we keep the most recent)
    pub uuid: Option<String>,
    /// An integer between 1-99. How much of the total priority fee + coinbase payment you want to be refunded for. This will negatively impact your prioritization because this refund is gonna eat into your bundle payment. Example: if a bundle pays 0.2 ETH of priority fee plus 1 ETH to coinbase, a refundPercent set to 50 will result in a transaction being appended after the bundle, paying 0.59 ETH back to the EOA. This is assuming the payout tx will cost beaver 0.01 ETH in fees, which are deduced from the 0.6 ETH payout.
    pub refund_percent: Option<u64>,
    /// You can specify an address that the funds from `refundPercent` will be sent to. If not specified, they will be sent to the `from` address of the first transaction
    pub refund_recipient: Option<Address>,
    /// The hashes of transactions in the bundle that the refund will be based on. If it's empty, we'll use the last transaction
    pub refund_transaction_hashes: Vec<TxHash>,
}

impl BeaverBundle {
    pub fn from_rlp_hex(txs: Vec<String>, block_number: BlockNumber) -> eyre::Result<Self> {
        Ok(Self {
            transactions: txs
                .iter()
                .map(|hex_string| {
                    hex::decode(hex_string)
                        .map_err(|e| eyre!("Invalid hexadecimal string: {e:?}"))
                        .and_then(|decoded_bytes| {
                            TransactionSigned::decode(&mut decoded_bytes.as_slice())
                                .map_err(|e| eyre!("Illegal RLP bytes for transaction: {e:?}"))
                        })
                })
                .collect::<Result<Vec<TransactionSigned>, _>>()?,
            block_number,
            ..Self::default()
        })
    }
}

impl Serialize for BeaverBundle {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("BeaverBundle", 2)?;
        state.serialize_field(
            "txs",
            &self
                .transactions
                .iter()
                .map(|tx| hex::encode(alloy_rlp::encode(tx)))
                .collect::<Vec<String>>(),
        )?;
        state.serialize_field("blockNumber", &format!("0x{:x}", self.block_number))?;

        if let Some(ref t) = self.min_timestamp {
            state.serialize_field("minTimestamp", t)?;
        }

        if let Some(ref t) = self.max_timestamp {
            state.serialize_field("maxTimestamp", t)?;
        }

        if !self.reverting_transaction_hashes.is_empty() {
            state.serialize_field(
                "revertingTxHashes",
                &self
                    .reverting_transaction_hashes
                    .iter()
                    .map(|hash| hash.to_string())
                    .collect::<Vec<String>>(),
            )?;
        }

        if !self.dropping_transaction_hashes.is_empty() {
            state.serialize_field(
                "droppingTxHashes",
                &self
                    .dropping_transaction_hashes
                    .iter()
                    .map(|hash| hash.to_string())
                    .collect::<Vec<String>>(),
            )?;
        }

        if let Some(ref uuid) = self.uuid {
            state.serialize_field("uuid", uuid)?;
        }

        if let Some(ref refund_pct) = self.refund_percent {
            state.serialize_field("refundPercent", refund_pct)?;
        }

        if let Some(ref refund_addr) = self.refund_recipient {
            state.serialize_field("refundRecipient", refund_addr)?;
        }

        if !self.refund_transaction_hashes.is_empty() {
            state.serialize_field(
                "refundTxHashes",
                &self
                    .refund_transaction_hashes
                    .iter()
                    .map(|hash| hash.to_string())
                    .collect::<Vec<String>>(),
            )?;
        }

        state.end()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_beaver_bundle_serialisation() {
        assert!(serde_json::to_string(&BeaverBundle::default()).is_ok());
        assert_eq!(
            serde_json::to_string(&BeaverBundle::default()).unwrap(),
            "{\"txs\":[],\"blockNumber\":\"0x0\"}".to_string()
        );

        assert!(serde_json::to_string(&BeaverBundle {
            transactions: vec![],
            block_number: 21862873,
            ..Default::default()
        })
        .is_ok());
        assert_eq!(
            serde_json::to_string(&BeaverBundle {
                transactions: vec![],
                block_number: 21862873,
                ..Default::default()
            })
            .unwrap(),
            "{\"txs\":[],\"blockNumber\":\"0x14d99d9\"}".to_string()
        );
        assert!(
            serde_json::to_string(&
                BeaverBundle::from_rlp_hex(vec!["0x02f8b20181948449bdee618501dcd6500083016b93942dabcea55a12d73191aece59f508b191fb68adac80b844095ea7b300000000000000000000000054e44dbb92dba848ace27f44c0cb4268981ef1cc00000000000000000000000000000000000000000000000052616e065f6915ebc080a0c497b6e53d7cb78e68c37f6186c8bb9e1b8a55c3e22462163495979b25c2caafa052769811779f438b73159c4cc6a05a889da8c1a16e432c2e37e3415c9a0b9887".to_string()], 21862873).unwrap()
        )
        .is_ok());
        assert_eq!(
            serde_json::to_string(&
                BeaverBundle::from_rlp_hex(vec!["0x02f8b20181948449bdee618501dcd6500083016b93942dabcea55a12d73191aece59f508b191fb68adac80b844095ea7b300000000000000000000000054e44dbb92dba848ace27f44c0cb4268981ef1cc00000000000000000000000000000000000000000000000052616e065f6915ebc080a0c497b6e53d7cb78e68c37f6186c8bb9e1b8a55c3e22462163495979b25c2caafa052769811779f438b73159c4cc6a05a889da8c1a16e432c2e37e3415c9a0b9887".to_string()], 21862873).unwrap()
        )
            .unwrap(),
            "{\"txs\":[\"0x02f8b20181948449bdee618501dcd6500083016b93942dabcea55a12d73191aece59f508b191fb68adac80b844095ea7b300000000000000000000000054e44dbb92dba848ace27f44c0cb4268981ef1cc00000000000000000000000000000000000000000000000052616e065f6915ebc080a0c497b6e53d7cb78e68c37f6186c8bb9e1b8a55c3e22462163495979b25c2caafa052769811779f438b73159c4cc6a05a889da8c1a16e432c2e37e3415c9a0b9887\"],\"blockNumber\":\"0x14d99d9\"}".to_string()
        );
    }
}
