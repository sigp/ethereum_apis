use crate::top_bid::TopBids;
use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::{
    async_trait,
    body::Body,
    extract::{FromRequest, Query, Request, State},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, RequestExt, Router,
};
use bytes::Bytes;
use futures::{sink::SinkExt, stream::StreamExt};
use http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
use relay_api_types::{
    ErrorResponse, GetDeliveredPayloadsQueryParams, GetReceivedBidsQueryParams,
    GetValidatorRegistrationQueryParams, SignedHeaderSubmission, SubmitBlockQueryParams,
    SubmitBlockRequest,
};
use serde::Serialize;
use std::net::SocketAddr;
use tracing::error;
use types::eth_spec::EthSpec;

use crate::{builder::Builder, data::Data, optimistic_v2::OptimisticV2};

/// Setup API Server.
pub fn new<I, A, E>(api_impl: I) -> Router
where
    E: EthSpec,
    I: AsRef<A> + Clone + Send + Sync + 'static,
    A: Builder<E> + Data + OptimisticV2<E> + TopBids + 'static,
{
    // build our application with a route
    Router::new()
        .route("/relay/v1/builder/blocks", post(submit_block::<I, A, E>))
        .route(
            "/relay/v1/builder/blocks_optimistic_v2",
            post(submit_block_optimistic_v2::<I, A, E>),
        )
        .route("/relay/v1/builder/headers", post(submit_header::<I, A, E>))
        .route(
            "/relay/v1/builder/validators",
            get(get_validators::<I, A, E>),
        )
        .route(
            "/relay/v1/data/bidtraces/builder_blocks_received",
            get(get_received_bids::<I, A>),
        )
        .route(
            "/relay/v1/data/bidtraces/proposer_payload_delivered",
            get(get_delivered_payloads::<I, A>),
        )
        .route(
            "/relay/v1/data/validator_registration",
            get(get_validator_registration::<I, A>),
        )
        .route("/relay/v1/builder/top_bids", get(get_top_bids::<I, A>))
        .with_state(api_impl)
}

async fn build_response<T>(result: Result<T, ErrorResponse>) -> Result<Response<Body>, StatusCode>
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

/// SubmitBlock - POST /relay/v1/builder/blocks
#[tracing::instrument(skip_all)]
async fn submit_block<I, A, E>(
    Query(query_params): Query<SubmitBlockQueryParams>,
    State(api_impl): State<I>,
    JsonOrSsz(body): JsonOrSsz<SubmitBlockRequest<E>>,
) -> Result<Response<Body>, StatusCode>
where
    E: EthSpec,
    I: AsRef<A> + Send + Sync,
    A: Builder<E>,
{
    let result = api_impl.as_ref().submit_block(query_params, body).await;
    build_response(result).await
}

/// SubmitBlockOptimisticV2 - POST /relay/v1/builder/blocks_optimistic_v2
#[tracing::instrument(skip_all)]
async fn submit_block_optimistic_v2<I, A, E>(
    Query(query_params): Query<SubmitBlockQueryParams>,
    State(api_impl): State<I>,
    JsonOrSsz(body): JsonOrSsz<SubmitBlockRequest<E>>,
) -> Result<Response<Body>, StatusCode>
where
    E: EthSpec,
    I: AsRef<A> + Send + Sync,
    A: OptimisticV2<E>,
{
    let result = api_impl
        .as_ref()
        .submit_block_optimistic_v2(query_params, body)
        .await;
    build_response(result).await
}

/// SubmitHeader - POST /relay/v1/builder/headers
#[tracing::instrument(skip_all)]
async fn submit_header<I, A, E>(
    Query(query_params): Query<SubmitBlockQueryParams>,
    State(api_impl): State<I>,
    JsonOrSsz(body): JsonOrSsz<SignedHeaderSubmission<E>>,
) -> Result<Response<Body>, StatusCode>
where
    E: EthSpec,
    I: AsRef<A> + Send + Sync,
    A: OptimisticV2<E>,
{
    let result = api_impl.as_ref().submit_header(query_params, body).await;
    build_response(result).await
}

/// GetValidators - GET /relay/v1/builder/validators
#[tracing::instrument(skip_all)]
async fn get_validators<I, A, E>(State(api_impl): State<I>) -> Result<Response<Body>, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: Builder<E>,
    E: EthSpec,
{
    let result = api_impl.as_ref().get_validators().await;
    build_response(result).await
}

/// GetDeliveredPayloads - GET /relay/v1/data/bidtraces/proposer_payload_delivered
#[tracing::instrument(skip_all)]
async fn get_delivered_payloads<I, A>(
    Query(query_params): Query<GetDeliveredPayloadsQueryParams>,
    State(api_impl): State<I>,
) -> Result<Response<Body>, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: Data,
{
    let result = api_impl.as_ref().get_delivered_payloads(query_params).await;
    build_response(result).await
}

/// GetReceivedBids - GET /relay/v1/data/bidtraces/builder_blocks_received
#[tracing::instrument(skip_all)]
async fn get_received_bids<I, A>(
    Query(query_params): Query<GetReceivedBidsQueryParams>,
    State(api_impl): State<I>,
) -> Result<Response<Body>, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: Data,
{
    let result = api_impl.as_ref().get_received_bids(query_params).await;
    build_response(result).await
}

/// GetValidatorRegistration - GET /relay/v1/data/validator_registration
#[tracing::instrument(skip_all)]
async fn get_validator_registration<I, A>(
    Query(query_params): Query<GetValidatorRegistrationQueryParams>,
    State(api_impl): State<I>,
) -> Result<Response<Body>, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: Data,
{
    let result = api_impl
        .as_ref()
        .get_validator_registration(query_params)
        .await;
    build_response(result).await
}

#[must_use]
#[derive(Debug, Clone, Copy, Default)]
struct Ssz<T>(T);

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
struct JsonOrSsz<T>(T);

#[async_trait]
impl<T, S> FromRequest<S> for JsonOrSsz<T>
where
    T: serde::de::DeserializeOwned + ssz::Decode + 'static,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let content_type_header = req.headers().get(CONTENT_TYPE);
        let content_type = content_type_header.and_then(|value| value.to_str().ok());

        if let Some(content_type) = content_type {
            if content_type.starts_with("application/json") {
                let Json(payload) = req.extract().await.map_err(IntoResponse::into_response)?;
                return Ok(Self(payload));
            }

            if content_type.starts_with("application/octet-stream") {
                let Ssz(payload) = req.extract().await.map_err(IntoResponse::into_response)?;
                return Ok(Self(payload));
            }
        }

        Err(StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response())
    }
}

/// GetTopBids - GET /relay/v1/builder/top_bids
#[tracing::instrument(skip_all)]
async fn get_top_bids<I, A>(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(api_impl): State<I>,
) -> impl IntoResponse
where
    I: AsRef<A> + Send + Sync + 'static,
    A: TopBids + 'static,
{
    ws.on_upgrade(move |socket| handle_socket(socket, addr, api_impl))
}

async fn handle_socket<I, A>(socket: WebSocket, who: SocketAddr, api_impl: I)
where
    I: AsRef<A> + Send + Sync + 'static,
    A: TopBids,
{
    let (mut sender, mut receiver) = socket.split();

    let mut send_task = tokio::spawn(async move {
        let stream = match api_impl.as_ref().get_top_bids().await {
            Ok(stream) => stream,
            Err(e) => {
                tracing::error!("Failed to get top bids stream: {:?}", e);
                let _ = sender.close().await;
                return;
            }
        };

        let mut stream = stream;
        while let Some(update) = stream.next().await {
            match serde_json::to_string(&update) {
                Ok(json) => {
                    if let Err(e) = sender.send(Message::Text(json)).await {
                        tracing::error!("Error sending message: {:?}", e);
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("Error serializing update: {:?}", e);
                    continue;
                }
            }
        }
        let _ = sender.close().await;
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(message)) = receiver.next().await {
            match message {
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    tracing::info!("Client {} disconnected", who);
}
