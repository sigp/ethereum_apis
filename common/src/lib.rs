use axum::{
    async_trait,
    body::Body,
    extract::{FromRequest, Request},
    response::{IntoResponse, Response},
};
use beacon_api_types::ForkVersionDeserialize;
use bytes::Bytes;
use flate2::read::GzDecoder;
use http::header::CONTENT_ENCODING;
use http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
use serde::{Deserialize, Serialize};
use std::io::Read;
use tracing::error;

pub const CONSENSUS_VERSION_HEADER: &'static str = "Eth-Consensus-Version";

pub async fn build_response<T>(
    result: Result<T, ErrorResponse>,
) -> Result<Response<Body>, StatusCode>
where
    T: Serialize + Send + 'static,
{
    let response_builder = Response::builder();

    let resp = match result {
        Ok(body) => {
            let mut response = response_builder.status(200);

            if let Some(response_headers) = response.headers_mut() {
                response_headers.insert(
                    CONTENT_TYPE,
                    HeaderValue::from_str("application/json").map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?,
                );
            }

            let body_content = tokio::task::spawn_blocking(move || {
                serde_json::to_vec(&body).map_err(|e| {
                    error!(error = ?e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })
            })
            .await
            .map_err(|e| {
                error!(error = ?e);
                StatusCode::INTERNAL_SERVER_ERROR
            })??;

            response.body(Body::from(body_content)).map_err(|e| {
                error!(error = ?e);
                StatusCode::INTERNAL_SERVER_ERROR
            })
        }
        Err(body) => {
            let mut response = response_builder.status(body.code);

            if let Some(response_headers) = response.headers_mut() {
                response_headers.insert(
                    CONTENT_TYPE,
                    HeaderValue::from_str("application/json").map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?,
                );
            }

            let body_content = tokio::task::spawn_blocking(move || {
                serde_json::to_vec(&body).map_err(|e| {
                    error!(error = ?e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })
            })
            .await
            .map_err(|e| {
                error!(error = ?e);
                StatusCode::INTERNAL_SERVER_ERROR
            })??;

            response.body(Body::from(body_content)).map_err(|e| {
                error!(error = ?e);
                StatusCode::INTERNAL_SERVER_ERROR
            })
        }
    };

    resp
}

#[must_use]
#[derive(Debug, Clone, Copy, Default)]
pub struct Ssz<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for Ssz<T>
where
    T: ssz::Decode,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let content_type_header = req.headers().get(CONTENT_TYPE);
        let content_type = content_type_header.and_then(|value| value.to_str().ok());

        if let Some(content_type) = content_type {
            if content_type.starts_with("application/octet-stream") {
                let bytes = Bytes::from_request(req, state)
                    .await
                    .map_err(IntoResponse::into_response)?;
                return Ok(T::from_ssz_bytes(&bytes)
                    .map(Ssz)
                    .map_err(|_| StatusCode::BAD_REQUEST.into_response())?);
            }
        }

        Err(StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response())
    }
}

#[must_use]
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonOrSsz<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for JsonOrSsz<T>
where
    T: serde::de::DeserializeOwned + ssz::Decode + 'static,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let headers = req.headers().clone();
        let content_type = headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok());

        let bytes = Bytes::from_request(req, _state)
            .await
            .map_err(IntoResponse::into_response)?;

        if let Some(content_type) = content_type {
            if content_type.starts_with(&ContentType::Json.to_string()) {
                let payload: T = serde_json::from_slice(&bytes)
                    .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
                return Ok(Self(payload));
            }

            if content_type.starts_with(&ContentType::Ssz.to_string()) {
                let payload = T::from_ssz_bytes(&bytes)
                    .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
                return Ok(Self(payload));
            }
        }

        Err(StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response())
    }
}

#[must_use]
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonOrSszMaybeGzipped<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for JsonOrSszMaybeGzipped<T>
where
    T: serde::de::DeserializeOwned + ssz::Decode + 'static,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let headers = req.headers().clone();
        let content_type = headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok());
        let content_encoding = headers
            .get(CONTENT_ENCODING)
            .and_then(|value| value.to_str().ok());

        let bytes = Bytes::from_request(req, _state)
            .await
            .map_err(IntoResponse::into_response)?;

        let decoded_bytes = if content_encoding == Some(&ContentEncoding::Gzip.to_string()) {
            let mut decoder = GzDecoder::new(&bytes[..]);
            let mut decoded = Vec::new();
            decoder
                .read_to_end(&mut decoded)
                .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
            decoded
        } else {
            bytes.to_vec()
        };

        if let Some(content_type) = content_type {
            if content_type.starts_with(&ContentType::Json.to_string()) {
                let payload: T = serde_json::from_slice(&decoded_bytes)
                    .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
                return Ok(Self(payload));
            }

            if content_type.starts_with(&ContentType::Ssz.to_string()) {
                let payload = T::from_ssz_bytes(&decoded_bytes)
                    .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
                return Ok(Self(payload));
            }
        }

        Err(StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response())
    }
}

// Headers
#[derive(Default)]
pub enum ContentType {
    #[default]
    Json,
    Ssz,
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentType::Json => write!(f, "application/json"),
            ContentType::Ssz => write!(f, "application/octet-stream"),
        }
    }
}

impl From<String> for ContentType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "application/json" => ContentType::Json,
            "application/octet-stream" => ContentType::Ssz,
            _ => panic!("unknown content type: {}", value),
        }
    }
}

#[derive(Default)]
pub enum ContentEncoding {
    Gzip,
    #[default]
    None,
}

impl std::fmt::Display for ContentEncoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentEncoding::Gzip => write!(f, "gzip"),
            ContentEncoding::None => write!(f, ""),
        }
    }
}

impl From<String> for ContentEncoding {
    fn from(value: String) -> Self {
        match value.as_ref() {
            "gzip" => ContentEncoding::Gzip,
            "" => ContentEncoding::None,
            _ => panic!("unknown content encoding: {}", value),
        }
    }
}

// Response types common
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: u16,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stacktraces: Option<Vec<String>>,
}

pub fn custom_internal_err(message: String) -> ErrorResponse {
    ErrorResponse {
        code: 500,
        message,
        stacktraces: None,
    }
}

#[must_use]
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonConsensusVersionHeader<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for JsonConsensusVersionHeader<T>
where
    T: ForkVersionDeserialize + 'static,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let headers = req.headers().clone();
        let fork_name = headers
            .get(CONSENSUS_VERSION_HEADER)
            .and_then(|value| value.to_str().ok())
            .and_then(|s| s.parse().ok())
            .ok_or(StatusCode::BAD_REQUEST.into_response())?;

        let bytes = Bytes::from_request(req, _state)
            .await
            .map_err(IntoResponse::into_response)?;

        let result = ForkVersionDeserialize::deserialize_by_fork::<serde_json::Value>(
            serde_json::de::from_slice(&bytes)
                .map_err(|_| StatusCode::BAD_REQUEST.into_response())?,
            fork_name,
        )
        .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
        Ok(Self(result))
    }
}
