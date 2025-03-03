//! RPC types that are supported by Beaverbuild
use alloy_primitives::{hex::FromHex, Address, BlockNumber, Bytes, TxHash};
use alloy_rpc_types_mev::EthSendBundle;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};

/// Bundle as recognised by Beaverbuild
///
/// Consult <https://beaverbuild.org/docs.html>. Note that the deprecated `replacementUuid` field
/// has been omitted.
#[serde_as]
#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct BeaverBundle {
    #[serde(flatten)]
    pub bundle: EthSendBundle,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// A list of transaction hashes contained in the bundle, that can be allowed to be removed from your bundle if it's deemed useful (but not revert)
    pub dropping_transaction_hashes: Vec<TxHash>,
    /// An integer between 1-99. How much of the total priority fee + coinbase payment you want to be refunded for. This will negatively impact your prioritization because this refund is gonna eat into your bundle payment. Example: if a bundle pays 0.2 ETH of priority fee plus 1 ETH to coinbase, a refundPercent set to 50 will result in a transaction being appended after the bundle, paying 0.59 ETH back to the EOA. This is assuming the payout tx will cost beaver 0.01 ETH in fees, which are deduced from the 0.6 ETH payout.
    pub refund_percent: Option<u64>,
    /// You can specify an address that the funds from `refundPercent` will be sent to. If not specified, they will be sent to the `from` address of the first transaction
    pub refund_recipient: Option<Address>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// The hashes of transactions in the bundle that the refund will be based on. If it's empty, we'll use the last transaction
    pub refund_transaction_hashes: Vec<TxHash>,
}

pub fn bundle_from_rlp_hex(
    txs: Vec<String>,
    block_number: BlockNumber,
) -> eyre::Result<EthSendBundle> {
    Ok(EthSendBundle {
        txs: txs
            .iter()
            .map(Bytes::from_hex)
            .collect::<Result<Vec<Bytes>, _>>()?,
        block_number,
        ..EthSendBundle::default()
    })
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
            bundle: EthSendBundle {
                txs: vec![],
                block_number: 21862873,
                ..Default::default()
            },
            ..Default::default()
        })
        .is_ok());
        assert_eq!(
            serde_json::to_string(&BeaverBundle {
                bundle: EthSendBundle {
                    txs: vec![],
                    block_number: 21862873,
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap(),
            "{\"txs\":[],\"blockNumber\":\"0x14d99d9\"}".to_string()
        );
        assert!(
            serde_json::to_string(&
                bundle_from_rlp_hex(vec!["0x02f8b20181948449bdee618501dcd6500083016b93942dabcea55a12d73191aece59f508b191fb68adac80b844095ea7b300000000000000000000000054e44dbb92dba848ace27f44c0cb4268981ef1cc00000000000000000000000000000000000000000000000052616e065f6915ebc080a0c497b6e53d7cb78e68c37f6186c8bb9e1b8a55c3e22462163495979b25c2caafa052769811779f438b73159c4cc6a05a889da8c1a16e432c2e37e3415c9a0b9887".to_string()], 21862873).unwrap()
        )
        .is_ok());
        assert_eq!(
            serde_json::to_string(&
                bundle_from_rlp_hex(vec!["0x02f8b20181948449bdee618501dcd6500083016b93942dabcea55a12d73191aece59f508b191fb68adac80b844095ea7b300000000000000000000000054e44dbb92dba848ace27f44c0cb4268981ef1cc00000000000000000000000000000000000000000000000052616e065f6915ebc080a0c497b6e53d7cb78e68c37f6186c8bb9e1b8a55c3e22462163495979b25c2caafa052769811779f438b73159c4cc6a05a889da8c1a16e432c2e37e3415c9a0b9887".to_string()], 21862873).unwrap()
        )
            .unwrap(),
            "{\"txs\":[\"0x02f8b20181948449bdee618501dcd6500083016b93942dabcea55a12d73191aece59f508b191fb68adac80b844095ea7b300000000000000000000000054e44dbb92dba848ace27f44c0cb4268981ef1cc00000000000000000000000000000000000000000000000052616e065f6915ebc080a0c497b6e53d7cb78e68c37f6186c8bb9e1b8a55c3e22462163495979b25c2caafa052769811779f438b73159c4cc6a05a889da8c1a16e432c2e37e3415c9a0b9887\"],\"blockNumber\":\"0x14d99d9\"}".to_string()
        );
    }
}
