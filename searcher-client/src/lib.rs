use alloy_rpc_types_mev::EthBundleHash;
use jsonrpsee::http_client::HttpClient;
use jsonrpsee::rpc_params;
use jsonrpsee_core::client::ClientT;
use url::Url;

use searcher_api_types::SendBundleRequest;

pub async fn send_bundle(url: Url, bundle: &SendBundleRequest) -> eyre::Result<EthBundleHash> {
    Ok(HttpClient::builder()
        .build(url)?
        .request("eth_sendBundle", rpc_params![bundle])
        .await?)
}

#[cfg(test)]
mod test {
    use super::*;

    use alloy_rpc_types_mev::EthSendBundle;
    use searcher_api_types::{bundle_from_rlp_hex, BeaverBundle, SendBundleRequest};

    const TEST_ENDPOINT: &str = "https://rpc.beaverbuild.org";

    fn test_endpoint() -> Url {
        TEST_ENDPOINT.parse().unwrap()
    }

    #[tokio::test]
    async fn test_send_bundle_beaver_rejects_empty_bundle() {
        let empty_bundle = SendBundleRequest::Beaver(BeaverBundle {
            bundle: EthSendBundle {
                txs: vec![],
                block_number: 0,
                ..EthSendBundle::default()
            },
            ..BeaverBundle::default()
        });
        let res = send_bundle(test_endpoint(), &empty_bundle).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_send_bundle_beaver_success() {
        let bundle = SendBundleRequest::Beaver(
            BeaverBundle { bundle: bundle_from_rlp_hex(vec!["0x02f8b20181948449bdee618501dcd6500083016b93942dabcea55a12d73191aece59f508b191fb68adac80b844095ea7b300000000000000000000000054e44dbb92dba848ace27f44c0cb4268981ef1cc00000000000000000000000000000000000000000000000052616e065f6915ebc080a0c497b6e53d7cb78e68c37f6186c8bb9e1b8a55c3e22462163495979b25c2caafa052769811779f438b73159c4cc6a05a889da8c1a16e432c2e37e3415c9a0b9887".to_string()], 1).expect("illegal RLP bytes for bundle"), ..BeaverBundle::default()}
        );
        let res = send_bundle(test_endpoint(), &bundle).await;
        assert!(res.is_ok());
        let resp = res.unwrap();
        assert_eq!(
            resp.bundle_hash.to_string(),
            "0xbfe05fa7cb2f9de981eeefe7246c9c9be6f69c3a3b33a05fdbf6afac42ddd294"
        );
    }
}
