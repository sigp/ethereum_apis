pub use relay_api_types::*;
use reqwest::{Client, Url};
use serde::Deserialize;

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    InvalidJson(serde_json::Error, String),
    ServerMessage(String),
    StatusCode(http::StatusCode),
    InvalidUrl(Url),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Reqwest(e)
    }
}

#[derive(Clone)]
pub struct RelayClient {
    client: Client,
    base_url: Url,
}

impl RelayClient {
    pub fn new(base_url: Url) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    async fn build_response<T>(&self, response: reqwest::Response) -> Result<T, Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        let status = response.status();
        let text = response.text().await;

        if status.is_success() {
            let text = text?;
            serde_json::from_str(&text).map_err(|e| Error::InvalidJson(e, text))
        } else if let Ok(message) = text {
            Err(Error::ServerMessage(message))
        } else {
            Err(Error::StatusCode(status))
        }
    }

    pub async fn submit_block<E>(
        &self,
        query_params: SubmitBlockQueryParams,
        body: SubmitBlockRequest<E>,
    ) -> Result<(), Error>
    where
        E: EthSpec,
    {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| Error::InvalidUrl(self.base_url.clone()))?
            .extend(&["relay", "v1", "builder", "blocks"]);
        let response = self
            .client
            .post(url)
            .query(&query_params)
            .json(&body)
            .send()
            .await?;

        self.build_response(response).await
    }

    pub async fn get_validators<E>(&self) -> Result<GetValidatorsResponse, Error>
    where
        E: EthSpec,
    {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| Error::InvalidUrl(self.base_url.clone()))?
            .extend(&["relay", "v1", "builder", "validators"]);
        let response = self.client.get(url).send().await?;

        self.build_response(response).await
    }

    pub async fn get_delivered_payloads(
        &self,
        query_params: GetDeliveredPayloadsQueryParams,
    ) -> Result<GetDeliveredPayloadsResponse, Error> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| Error::InvalidUrl(self.base_url.clone()))?
            .extend(&[
                "relay",
                "v1",
                "data",
                "bidtraces",
                "proposer_payload_delivered",
            ]);
        let response = self.client.get(url).query(&query_params).send().await?;

        self.build_response(response).await
    }

    pub async fn get_received_bids(
        &self,
        query_params: GetReceivedBidsQueryParams,
    ) -> Result<GetReceivedBidsResponse, Error> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| Error::InvalidUrl(self.base_url.clone()))?
            .extend(&[
                "relay",
                "v1",
                "data",
                "bidtraces",
                "builder_blocks_received",
            ]);
        let response = self.client.get(url).query(&query_params).send().await?;

        self.build_response(response).await
    }

    pub async fn get_validator_registration(
        &self,
        query_params: GetValidatorRegistrationQueryParams,
    ) -> Result<GetValidatorRegistrationResponse, Error> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| Error::InvalidUrl(self.base_url.clone()))?
            .extend(&["relay", "v1", "data", "validator_registration"]);
        let response = self.client.get(url).query(&query_params).send().await?;

        self.build_response(response).await
    }
}
