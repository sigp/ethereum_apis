use async_trait::async_trait;
use ethereum_apis_common::{build_response, ErrorResponse};
use reqwest::Client;
use reqwest::Url;
use serde::de::DeserializeOwned;
use types::{
    builder_bid::SignedBuilderBid, eth_spec::EthSpec, ExecutionBlockHash, ExecutionPayload,
    ForkName, PublicKeyBytes, SignedBlindedBeaconBlock, SignedValidatorRegistrationData, Slot,
};

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    InvalidJson(serde_json::Error, String),
    ServerMessage(ErrorResponse),
    StatusCode(reqwest::StatusCode),
    InvalidUrl(Url),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Reqwest(e)
    }
}

#[derive(Clone)]
pub struct BuilderClient {
    client: Client,
    base_url: Url,
}

impl BuilderClient {
    pub fn new(base_url: Url) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    async fn build_response<T>(&self, response: reqwest::Response) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let status = response.status();
        let text = response.text().await?;

        if status.is_success() {
            serde_json::from_str(&text).map_err(|e| Error::InvalidJson(e, text))
        } else {
            Err(Error::ServerMessage(
                serde_json::from_str(&text).map_err(|e| Error::InvalidJson(e, text))?,
            ))
        }
    }

    pub async fn register_validators(
        &self,
        registrations: Vec<SignedValidatorRegistrationData>,
    ) -> Result<(), Error> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| Error::InvalidUrl(self.base_url.clone()))?
            .extend(&["eth", "v1", "builder", "validators"]);

        let response = self.client.post(url).json(&registrations).send().await?;

        self.build_response(response).await
    }

    pub async fn submit_blinded_block<E: EthSpec>(
        &self,
        block: SignedBlindedBeaconBlock<E>,
    ) -> Result<ExecutionPayload<E>, Error> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| Error::InvalidUrl(self.base_url.clone()))?
            .extend(&["eth", "v1", "builder", "blinded_blocks"]);

        let response = self.client.post(url).json(&block).send().await?;

        self.build_response(response).await
    }

    pub async fn get_header<E: EthSpec>(
        &self,
        slot: Slot,
        parent_hash: ExecutionBlockHash,
        pubkey: PublicKeyBytes,
    ) -> Result<SignedBuilderBid<E>, Error> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| Error::InvalidUrl(self.base_url.clone()))?
            .extend(&[
                "eth",
                "v1",
                "builder",
                "header",
                &slot.to_string(),
                &parent_hash.to_string(),
                &pubkey.to_string(),
            ]);

        let response = self.client.get(url).send().await?;

        self.build_response(response).await
    }

    pub async fn get_status(&self) -> Result<(), Error> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| Error::InvalidUrl(self.base_url.clone()))?
            .extend(&["eth", "v1", "builder", "status"]);

        let response = self.client.get(url).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(Error::StatusCode(response.status()))
        }
    }
}

