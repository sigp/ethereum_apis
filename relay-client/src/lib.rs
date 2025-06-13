pub use ethereum_apis_common::{ContentEncoding, ContentType, ErrorResponse};
use futures::{Stream, StreamExt};
use http::header::InvalidHeaderValue;
use http::header::CONTENT_ENCODING;
use http::header::CONTENT_TYPE;
use http::HeaderValue;
pub use relay_api_types::*;
use reqwest::Client;
use reqwest::Url;
use serde::Deserialize;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    InvalidJson(serde_json::Error, String),
    ServerMessage(ErrorResponse),
    StatusCode(http::StatusCode),
    InvalidUrl(Url),
    WebSocket(tokio_tungstenite::tungstenite::Error),
    InvalidHeader(InvalidHeaderValue),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Reqwest(e)
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for Error {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        Error::WebSocket(e)
    }
}

impl From<InvalidHeaderValue> for Error {
    fn from(value: InvalidHeaderValue) -> Self {
        Error::InvalidHeader(value)
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
        self.build_response_with_headers(response, <_>::default(), <_>::default())
            .await
    }

    async fn build_response_with_headers<T>(
        &self,
        mut response: reqwest::Response,
        content_type: ContentType,
        content_encoding: ContentEncoding,
    ) -> Result<T, Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        response.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_str(content_type.to_string().as_str())?,
        );

        match content_encoding {
            ContentEncoding::Gzip => {
                response.headers_mut().insert(
                    CONTENT_ENCODING,
                    HeaderValue::from_str(content_encoding.to_string().as_str())?,
                );
            }
            ContentEncoding::None => {}
        }

        let status = response.status();
        let text = response.text().await;

        if status.is_success() {
            let text = text?;
            serde_json::from_str(&text).map_err(|e| Error::InvalidJson(e, text))
        } else if let Ok(message) = text {
            Err(Error::ServerMessage(
                serde_json::from_str(&message).map_err(|e| Error::InvalidJson(e, message))?,
            ))
        } else {
            Err(Error::StatusCode(status))
        }
    }

    pub async fn submit_block<E>(
        &self,
        query_params: &SubmitBlockQueryParams,
        body: &SubmitBlockRequest,
        content_type: ContentType,
        content_encoding: ContentEncoding,
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
            .query(query_params)
            .json(body)
            .send()
            .await?;

        self.build_response_with_headers(response, content_type, content_encoding)
            .await
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
        query_params: &GetDeliveredPayloadsQueryParams,
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
        let response = self.client.get(url).query(query_params).send().await?;

        self.build_response(response).await
    }

    pub async fn get_received_bids(
        &self,
        query_params: &GetReceivedBidsQueryParams,
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
        let response = self.client.get(url).query(query_params).send().await?;

        self.build_response(response).await
    }

    pub async fn get_validator_registration(
        &self,
        query_params: &GetValidatorRegistrationQueryParams,
    ) -> Result<GetValidatorRegistrationResponse, Error> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| Error::InvalidUrl(self.base_url.clone()))?
            .extend(&["relay", "v1", "data", "validator_registration"]);
        let response = self.client.get(url).query(query_params).send().await?;

        self.build_response(response).await
    }

    pub async fn submit_header<E>(
        &self,
        query_params: &SubmitBlockQueryParams,
        body: &SignedHeaderSubmission<E>,
        content_type: ContentType,
        content_encoding: ContentEncoding,
    ) -> Result<(), Error>
    where
        E: EthSpec,
    {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| Error::InvalidUrl(self.base_url.clone()))?
            .extend(&["relay", "v1", "builder", "headers"]);
        let response = self
            .client
            .post(url)
            .query(query_params)
            .json(body)
            .send()
            .await?;

        self.build_response_with_headers(response, content_type, content_encoding)
            .await
    }

    pub async fn submit_block_optimistic_v2<E>(
        &self,
        query_params: &SubmitBlockQueryParams,
        body: &SubmitBlockRequest,
        content_type: ContentType,
        content_encoding: ContentEncoding,
    ) -> Result<(), Error>
    where
        E: EthSpec,
    {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| Error::InvalidUrl(self.base_url.clone()))?
            .extend(&["relay", "v1", "builder", "blocks_optimistic_v2"]);
        let response = self
            .client
            .post(url)
            .query(query_params)
            .json(body)
            .send()
            .await?;

        self.build_response_with_headers(response, content_type, content_encoding)
            .await
    }

    pub async fn submit_cancellation(
        &self,
        body: &SignedCancellation,
        content_type: ContentType,
        content_encoding: ContentEncoding,
    ) -> Result<(), Error> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| Error::InvalidUrl(self.base_url.clone()))?
            .extend(&["relay", "v1", "builder", "cancel_bid"]);
        let response = self.client.post(url).json(body).send().await?;

        self.build_response_with_headers(response, content_type, content_encoding)
            .await
    }

    pub async fn subscribe_top_bids(
        &self,
    ) -> Result<impl Stream<Item = Result<TopBidUpdate, Error>>, Error> {
        let mut url = self.base_url.clone();
        url.set_path("/relay/v1/builder/top_bids");

        let ws_scheme = match url.scheme() {
            "http" => "ws",
            "https" => "wss",
            _ => return Err(Error::InvalidUrl(self.base_url.clone())),
        };
        url.set_scheme(ws_scheme)
            .map_err(|_| Error::InvalidUrl(self.base_url.clone()))?;

        let (ws_stream, _) = connect_async(url.as_str())
            .await
            .map_err(Error::WebSocket)?;
        let (_, read) = ws_stream.split();

        let stream = read.filter_map(|message| async {
            match message {
                Ok(Message::Text(text)) => match serde_json::from_str::<TopBidUpdate>(&text) {
                    Ok(update) => Some(Ok(update)),
                    Err(e) => Some(Err(Error::InvalidJson(e, text.as_str().to_string()))),
                },
                Ok(Message::Binary(bin)) => match serde_json::from_slice::<TopBidUpdate>(&bin) {
                    Ok(update) => Some(Ok(update)),
                    Err(e) => {
                        let text = String::from_utf8_lossy(&bin).to_string();
                        Some(Err(Error::InvalidJson(e, text)))
                    }
                },
                _ => None, // Ignore other message types
            }
        });
        Ok(stream)
    }
}
