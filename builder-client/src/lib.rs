use axum::http::HeaderMap;
use axum::http::HeaderValue;
use builder_api_types::*;
pub use builder_bid::SignedBuilderBid;
use ethereum_apis_common::ContentType;
pub use ethereum_apis_common::ErrorResponse;
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use reqwest::Client;
use reqwest::Url;
use serde::de::DeserializeOwned;
use ssz::DecodeError;
use ssz::Encode;

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    InvalidJson(serde_json::Error, String),
    InvalidSsz(DecodeError),
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
        Self { client: Client::new(), base_url }
    }

    async fn build_response_with_headers<T>(
        &self,
        response: reqwest::Response,
        content_type: ContentType,
        fork_name: ForkName,
    ) -> Result<T, Error>
    where
        T: DeserializeOwned + ForkVersionDecode,
    {
        let status = response.status();
        let text = response.text().await?;

        if status.is_success() {
            match content_type {
                ContentType::Json => {
                    serde_json::from_str(&text).map_err(|e| Error::InvalidJson(e, text))
                }
                ContentType::Ssz => {
                    T::from_ssz_bytes_by_fork(text.as_bytes(), fork_name).map_err(Error::InvalidSsz)
                }
            }
        } else {
            Err(Error::ServerMessage(
                serde_json::from_str(&text).map_err(|e| Error::InvalidJson(e, text))?,
            ))
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
        registrations: &[SignedValidatorRegistrationData],
    ) -> Result<(), Error> {
        let mut url = self.base_url.clone();
        url.path_segments_mut().map_err(|_| Error::InvalidUrl(self.base_url.clone()))?.extend(&[
            "eth",
            "v1",
            "builder",
            "validators",
        ]);

        let response = self.client.post(url).json(registrations).send().await?;

        self.build_response(response).await
    }

    pub async fn submit_blinded_block<E: EthSpec>(
        &self,
        block: &SignedBlindedBeaconBlock<E>,
        content_type: ContentType,
        fork_name: ForkName,
    ) -> Result<ExecutionPayload<E>, Error> {
        let mut url = self.base_url.clone();
        url.path_segments_mut().map_err(|_| Error::InvalidUrl(self.base_url.clone()))?.extend(&[
            "eth",
            "v1",
            "builder",
            "blinded_blocks",
        ]);

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_str(&content_type.to_string()).unwrap());

        let response = match content_type {
            ContentType::Json => self.client.post(url).headers(headers).json(block).send().await?,
            ContentType::Ssz => {
                self.client.post(url).headers(headers).body(block.as_ssz_bytes()).send().await?
            }
        };

        self.build_response_with_headers(response, content_type, fork_name).await
    }

    pub async fn get_header<E: EthSpec>(
        &self,
        slot: Slot,
        parent_hash: ExecutionBlockHash,
        pubkey: &PublicKeyBytes,
        content_type: ContentType,
        fork_name: ForkName,
    ) -> Result<SignedBuilderBid<E>, Error> {
        let mut url = self.base_url.clone();
        url.path_segments_mut().map_err(|_| Error::InvalidUrl(self.base_url.clone()))?.extend(&[
            "eth",
            "v1",
            "builder",
            "header",
            &slot.to_string(),
            &parent_hash.to_string(),
            &pubkey.to_string(),
        ]);

        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_str(&content_type.to_string()).unwrap());

        let response = self.client.get(url).headers(headers).send().await?;

        self.build_response_with_headers(response, content_type, fork_name).await
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
