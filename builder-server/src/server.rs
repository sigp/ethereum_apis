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
    eth_spec::EthSpec, fork_versioned_response::ForkVersionDecode, ExecutionBlockHash, ForkName,
    FullPayloadContents, MainnetEthSpec, PublicKeyBytes, SignedBlindedBeaconBlock,
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
            "/eth/v1/builder/header/{slot}/{parent_hash}/{pubkey}",
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
    dbg!(&headers);
    let content_type_header = headers.get(CONTENT_TYPE);
    let content_type = content_type_header.and_then(|value| value.to_str().ok());
    let content_type = match content_type {
        Some("application/octet-stream") => ContentType::Ssz,
        _ => ContentType::Json,
    };
    let slot = block.slot();

    let res = api_impl.as_ref().submit_blinded_block(block).await;

    println!("in submit_blinded_block");
    let response =
        build_response_with_headers(res, content_type, api_impl.as_ref().fork_name_at_slot(slot))
            .await;
    println!("Got response ok {}", response.is_ok());
    response
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
        }
        _ => {
            info!("REQUESTED JSON");
            ContentType::Json
        }
    };

    let res = api_impl
        .as_ref()
        .get_header(slot, parent_hash, pubkey)
        .await;
    tracing::info!("Got response from builder, constructing response");
    build_response_with_headers(res, content_type, api_impl.as_ref().fork_name_at_slot(slot)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::{body::Body, http::Request};
    use builder_api_types::{
        builder_bid::SignedBuilderBid, BeaconBlock, BeaconBlockDeneb, Blob, BlobsBundle,
        EmptyBlock, ExecutionPayload, ExecutionPayloadAndBlobs, ExecutionPayloadDeneb, ForkName,
        ForkVersionDecode, FullPayloadContents, KzgCommitment, KzgProof, MainnetEthSpec, Signature,
    };
    use ethereum_apis_common::{ErrorResponse, CONSENSUS_VERSION_HEADER};
    use http::HeaderValue;
    use ssz::Encode;
    use std::{marker::PhantomData, usize};
    use tower::ServiceExt;

    #[derive(Clone)]
    struct DummyBuilder<E: EthSpec> {
        _phantom: PhantomData<E>,
    }

    impl<E: EthSpec> AsRef<DummyBuilder<E>> for DummyBuilder<E> {
        fn as_ref(&self) -> &DummyBuilder<E> {
            self
        }
    }

    #[async_trait]
    impl<E: EthSpec> Builder<E> for DummyBuilder<E> {
        fn fork_name_at_slot(&self, _slot: Slot) -> builder_api_types::ForkName {
            ForkName::Deneb
        }

        async fn get_header(
            &self,
            _slot: Slot,
            _parent_hash: ExecutionBlockHash,
            _pubkey: PublicKeyBytes,
        ) -> Result<SignedBuilderBid<E>, ErrorResponse> {
            todo!()
        }

        async fn register_validators(
            &self,
            _registrations: Vec<SignedValidatorRegistrationData>,
        ) -> Result<(), ErrorResponse> {
            Ok(())
        }

        async fn submit_blinded_block(
            &self,
            _block: SignedBlindedBeaconBlock<E>,
        ) -> Result<FullPayloadContents<E>, ErrorResponse> {
            let payload_and_blobs: ExecutionPayloadAndBlobs<E> = ExecutionPayloadAndBlobs {
                blobs_bundle: BlobsBundle {
                    commitments: vec![KzgCommitment::empty_for_testing()].into(),
                    proofs: vec![KzgProof::empty()].into(),
                    blobs: vec![Blob::<E>::new(vec![42; E::bytes_per_blob()]).unwrap()].into(),
                },
                execution_payload: ExecutionPayload::Deneb(ExecutionPayloadDeneb {
                    ..Default::default()
                }),
            };
            let full_payload = FullPayloadContents::PayloadAndBlobs(payload_and_blobs);
            Ok(full_payload)
        }
    }

    #[tokio::test]
    async fn test_api() {
        let app = new(DummyBuilder::<MainnetEthSpec> {
            _phantom: PhantomData,
        });

        let spec = MainnetEthSpec::default_spec();
        let dummy_block = SignedBlindedBeaconBlock::<MainnetEthSpec>::from_block(
            BeaconBlock::Deneb(BeaconBlockDeneb::empty(&spec)),
            Signature::empty(),
        );
        let request = Request::builder()
            .uri("/eth/v1/builder/blinded_blocks")
            .method("POST")
            .header(
                CONTENT_TYPE,
                HeaderValue::from_static("application/octet-stream"),
            )
            .header(CONSENSUS_VERSION_HEADER, HeaderValue::from_static("deneb"))
            .body(Body::from(dummy_block.as_ssz_bytes()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Assert status code
        // assert_eq!(response.status(), StatusCode::ACCEPTED);

        // Get response body as bytes
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();

        dbg!(
            FullPayloadContents::<MainnetEthSpec>::from_ssz_bytes_by_fork(&body, ForkName::Deneb)
                .unwrap()
        );
    }
}
