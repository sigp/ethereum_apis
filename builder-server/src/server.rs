use axum::{
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Json, Router,
};
use builder_api_types::{
    eth_spec::EthSpec, fork_versioned_response::EmptyMetadata, ExecutionBlockHash,
    ForkVersionedResponse, PublicKeyBytes, SignedBlindedBeaconBlock,
    SignedValidatorRegistrationData, Slot,
};
use ethereum_apis_common::build_response;

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
    State(api_impl): State<I>,
    Json(block): Json<SignedBlindedBeaconBlock<E>>,
) -> Result<Response<Body>, StatusCode>
where
    E: EthSpec,
    I: AsRef<A> + Send + Sync,
    A: Builder<E>,
{
    let res = api_impl
        .as_ref()
        .submit_blinded_block(block)
        .await
        .map(|payload| ForkVersionedResponse {
            version: Some(payload.fork_name()),
            metadata: EmptyMetadata {},
            data: payload,
        });
    build_response(res).await
}

async fn get_status() -> StatusCode {
    StatusCode::OK
}

async fn get_header<I, A, E>(
    State(api_impl): State<I>,
    Path((slot, parent_hash, pubkey)): Path<(Slot, ExecutionBlockHash, PublicKeyBytes)>,
) -> Result<Response<Body>, StatusCode>
where
    E: EthSpec,
    I: AsRef<A> + Send + Sync,
    A: Builder<E>,
{
    let res = api_impl
        .as_ref()
        .get_header(slot, parent_hash, pubkey)
        .await
        .map(|signed_bid| ForkVersionedResponse {
            version: Some(api_impl.as_ref().fork_name_at_slot(slot)),
            metadata: EmptyMetadata {},
            data: signed_bid,
        });
    build_response(res).await
}
