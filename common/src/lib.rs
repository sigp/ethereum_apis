use axum::{
    body::Body,
    extract::{FromRequest, Request},
    response::{IntoResponse, Response},
};
use beacon_api_types::{
    beacon_response::EmptyMetadata, ContextDeserialize, ForkName, ForkVersionDecode,
    ForkVersionedResponse,
};
use bytes::Bytes;
use flate2::read::GzDecoder;
use http::header::CONTENT_ENCODING;
use http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
use mediatype::{names, MediaType, MediaTypeList};
use serde::{Deserialize, Serialize};
use ssz::Encode;
use std::{fmt, io::Read, str::FromStr};
use tracing::error;

pub const CONSENSUS_VERSION_HEADER: &str = "Eth-Consensus-Version";

pub fn build_response_with_headers<T>(
    result: Result<T, ErrorResponse>,
    content_type: ContentType,
    fork_name: ForkName,
) -> Result<Response<Body>, StatusCode>
where
    T: Serialize + Encode + Send + 'static,
{
    let response_builder = Response::builder();

    let resp = match result {
        Ok(body) => {
            let mut response = response_builder.status(200);

            if let Some(response_headers) = response.headers_mut() {
                response_headers.insert(
                    CONSENSUS_VERSION_HEADER,
                    HeaderValue::from_str(&fork_name.to_string()).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?,
                );

                response_headers.insert(
                    CONTENT_TYPE,
                    HeaderValue::from_str(&content_type.to_string()).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?,
                );
            }
            let body_content = match content_type {
                ContentType::Json => {
                    let body = ForkVersionedResponse {
                        version: fork_name,
                        metadata: EmptyMetadata {},
                        data: body,
                    };
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?
                }

                ContentType::Ssz => T::as_ssz_bytes(&body),
            };
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
                    HeaderValue::from_str(&content_type.to_string()).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?,
                );
            }

            let body_content = serde_json::to_vec(&body).map_err(|e| {
                error!(error = ?e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            response.body(Body::from(body_content)).map_err(|e| {
                error!(error = ?e);
                StatusCode::INTERNAL_SERVER_ERROR
            })
        }
    };

    resp
}

pub fn build_response<T>(result: Result<T, ErrorResponse>) -> Result<Response<Body>, StatusCode>
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

            let body_content = serde_json::to_vec(&body).map_err(|e| {
                error!(error = ?e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

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

            let body_content = serde_json::to_vec(&body).map_err(|e| {
                error!(error = ?e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

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
                let bytes =
                    Bytes::from_request(req, state).await.map_err(IntoResponse::into_response)?;
                return T::from_ssz_bytes(&bytes)
                    .map(Ssz)
                    .map_err(|_| StatusCode::BAD_REQUEST.into_response());
            }
        }

        Err(StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response())
    }
}

#[must_use]
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonOrSszWithFork<T>(pub T);

impl<T, S> FromRequest<S> for JsonOrSszWithFork<T>
where
    T: serde::de::DeserializeOwned + ForkVersionDecode + 'static,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let headers = req.headers().clone();
        let content_type = headers.get(CONTENT_TYPE).and_then(|value| value.to_str().ok());
        let fork_name = headers
            .get(CONSENSUS_VERSION_HEADER)
            .and_then(|value| ForkName::from_str(value.to_str().unwrap()).ok());

        let bytes = Bytes::from_request(req, _state).await.map_err(IntoResponse::into_response)?;

        if let Some(content_type) = content_type {
            if content_type.starts_with(&ContentType::Json.to_string()) {
                let payload: T = serde_json::from_slice(&bytes)
                    .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
                return Ok(Self(payload));
            }

            if content_type.starts_with(&ContentType::Ssz.to_string()) {
                let payload = T::from_ssz_bytes_by_fork(&bytes, fork_name.unwrap())
                    .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
                return Ok(Self(payload));
            }
        }

        Err(StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response())
    }
}

#[must_use]
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonOrSsz<T>(pub T);

impl<T, S> FromRequest<S> for JsonOrSsz<T>
where
    T: serde::de::DeserializeOwned + ssz::Decode + 'static,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let headers = req.headers().clone();
        let content_type = headers.get(CONTENT_TYPE).and_then(|value| value.to_str().ok());

        let bytes = Bytes::from_request(req, _state).await.map_err(IntoResponse::into_response)?;

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

impl<T, S> FromRequest<S> for JsonOrSszMaybeGzipped<T>
where
    T: serde::de::DeserializeOwned + ssz::Decode + 'static,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let headers = req.headers().clone();
        let content_type = headers.get(CONTENT_TYPE).and_then(|value| value.to_str().ok());
        let content_encoding = headers.get(CONTENT_ENCODING).and_then(|value| value.to_str().ok());

        let bytes = Bytes::from_request(req, _state).await.map_err(IntoResponse::into_response)?;

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
#[derive(Default, Clone, Copy, Debug)]
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
            _ => panic!("unknown content type: {value}"),
        }
    }
}

#[derive(Default, Clone, Copy)]
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
            _ => panic!("unknown content encoding: {value}"),
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
    ErrorResponse { code: 500, message, stacktraces: None }
}

#[must_use]
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonConsensusVersionHeader<T>(pub T);

impl<T, S> FromRequest<S> for JsonConsensusVersionHeader<T>
where
    T: ContextDeserialize<'static, ForkName>,
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

        let bytes = Bytes::from_request(req, _state).await.map_err(IntoResponse::into_response)?;

        let result = ContextDeserialize::context_deserialize::<serde_json::Value>(
            serde_json::de::from_slice(&bytes)
                .map_err(|_| StatusCode::BAD_REQUEST.into_response())?,
            fork_name,
        )
        .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
        Ok(Self(result))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Accept {
    Json,
    Ssz,
    Any,
}

impl fmt::Display for Accept {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Accept::Ssz => write!(f, "application/octet-stream"),
            Accept::Json => write!(f, "application/json"),
            Accept::Any => write!(f, "*/*"),
        }
    }
}

impl FromStr for Accept {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let media_type_list = MediaTypeList::new(s);

        // [q-factor weighting]: https://datatracker.ietf.org/doc/html/rfc7231#section-5.3.2
        // find the highest q-factor supported accept type
        let mut highest_q = 0_u16;
        let mut accept_type = None;

        const APPLICATION: &str = names::APPLICATION.as_str();
        const OCTET_STREAM: &str = names::OCTET_STREAM.as_str();
        const JSON: &str = names::JSON.as_str();
        const STAR: &str = names::_STAR.as_str();
        const Q: &str = names::Q.as_str();

        media_type_list.into_iter().for_each(|item| {
            if let Ok(MediaType { ty, subty, suffix: _, params }) = item {
                let q_accept = match (ty.as_str(), subty.as_str()) {
                    (APPLICATION, OCTET_STREAM) => Some(Accept::Ssz),
                    (APPLICATION, JSON) => Some(Accept::Json),
                    (STAR, STAR) => Some(Accept::Any),
                    _ => None,
                }
                .map(|item_accept_type| {
                    let q_val = params
                        .iter()
                        .find_map(|(n, v)| match n.as_str() {
                            Q => {
                                Some((v.as_str().parse::<f32>().unwrap_or(0_f32) * 1000_f32) as u16)
                            }
                            _ => None,
                        })
                        .or(Some(1000_u16));

                    (q_val.unwrap(), item_accept_type)
                });

                match q_accept {
                    Some((q, accept)) if q > highest_q => {
                        highest_q = q;
                        accept_type = Some(accept);
                    }
                    _ => (),
                }
            }
        });
        accept_type.ok_or_else(|| "accept header is not supported".to_string())
    }
}
