use std::str::FromStr;

use axum::{
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Json, Router,
};
use builder_api_types::{
    eth_spec::EthSpec, ExecutionBlockHash, PublicKeyBytes, SignedBlindedBeaconBlock,
    SignedValidatorRegistrationData, Slot,
};
use ethereum_apis_common::{
    build_response, build_response_with_headers, Accept, ContentType, JsonOrSszWithFork,
};
use http::{
    header::{ACCEPT, CONTENT_TYPE},
    HeaderMap,
};
use tracing::info;

use crate::builder::Builder;

pub fn new<I, A, E>(api_impl: I) -> Router
where
    E: EthSpec,
    I: AsRef<A> + Clone + Send + Sync + 'static,
    A: Builder<E> + 'static,
{
    Router::new()
        .route(
            "/eth/v1/builder/validators",
            post(register_validators::<I, A, E>),
        )
        .route(
            "/eth/v1/builder/blinded_blocks",
            post(submit_blinded_block::<I, A, E>),
        )
        .route("/eth/v1/builder/status", get(get_status))
        .route(
            "/eth/v1/builder/header/:slot/:parent_hash/:pubkey",
            get(get_header::<I, A, E>),
        )
        .with_state(api_impl)
}

async fn register_validators<I, A, E>(
    State(api_impl): State<I>,
    Json(registrations): Json<Vec<SignedValidatorRegistrationData>>,
) -> Result<Response<Body>, StatusCode>
where
    E: EthSpec,
    I: AsRef<A> + Send + Sync,
    A: Builder<E>,
{
    let res = api_impl.as_ref().register_validators(registrations).await;
    build_response(res).await
}

async fn submit_blinded_block<I, A, E>(
    headers: HeaderMap,
    State(api_impl): State<I>,
    JsonOrSszWithFork(block): JsonOrSszWithFork<SignedBlindedBeaconBlock<E>>,
) -> Result<Response<Body>, StatusCode>
where
    E: EthSpec,
    I: AsRef<A> + Send + Sync,
    A: Builder<E>,
{
    let content_type_header = headers.get(CONTENT_TYPE);
    let content_type = content_type_header.and_then(|value| value.to_str().ok());
    let content_type = match content_type {
        Some("application/octet-stream") => ContentType::Ssz,
        _ => ContentType::Json,
    };
    let slot = block.slot();
    let res = api_impl.as_ref().submit_blinded_block(block).await;

    build_response_with_headers(res, content_type, api_impl.as_ref().fork_name_at_slot(slot)).await
}

async fn get_status() -> StatusCode {
    StatusCode::OK
}

async fn get_header<I, A, E>(
    headers: HeaderMap,
    State(api_impl): State<I>,
    Path((slot, parent_hash, pubkey)): Path<(Slot, ExecutionBlockHash, PublicKeyBytes)>,
) -> Result<Response<Body>, StatusCode>
where
    E: EthSpec,
    I: AsRef<A> + Send + Sync,
    A: Builder<E>,
{
    let content_type_header = headers.get(ACCEPT);
    let content_type_str = content_type_header
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/json");
    let content_type = match Accept::from_str(content_type_str) {
        Ok(Accept::Ssz) => {
            info!("REQUESTED SSZ");
            ContentType::Ssz
        },
        _ => {
            info!("REQUESTED JSON");
            ContentType::Json
        },
    };

    let res = api_impl
        .as_ref()
        .get_header(slot, parent_hash, pubkey)
        .await;
    build_response_with_headers(res, content_type, api_impl.as_ref().fork_name_at_slot(slot)).await
}
