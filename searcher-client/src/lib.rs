use alloy_rpc_types_mev::EthBundleHash;
use reqwest::Client;
use url::Url;

use searcher_api_types::SendBundleRequest;

pub async fn send_bundle(url: Url, bundle: &SendBundleRequest) -> eyre::Result<EthBundleHash> {
    Ok(serde_json::from_str(
        &Client::new()
            .post(url)
            .json(bundle)
            .send()
            .await?
            .text()
            .await?,
    )?)
}
