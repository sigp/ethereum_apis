use std::str::FromStr;

use axum::{
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Router,
};
use builder_api_types::{
    EthSpec, ExecutionBlockHash, PublicKeyBytes, SignedBlindedBeaconBlock,
    SignedValidatorRegistrationData, Slot, VariableList,
};
use ethereum_apis_common::{
    build_response, build_response_with_headers, Accept, ContentType, JsonOrSsz, JsonOrSszWithFork,
};
use http::{
    header::{ACCEPT, CONTENT_TYPE},
    HeaderMap,
};

pub type ValidatorRegistrations<E> =
    VariableList<SignedValidatorRegistrationData, <E as EthSpec>::ValidatorRegistryLimit>;

use crate::builder::Builder;

pub fn new<I, A, E>(api_impl: I) -> Router
where
    E: EthSpec,
    I: AsRef<A> + Clone + Send + Sync + 'static,
    A: Builder<E> + 'static,
{
    Router::new()
        .route("/eth/v1/builder/validators", post(register_validators::<I, A, E>))
        .route("/eth/v1/builder/blinded_blocks", post(submit_blinded_block::<I, A, E>))
        .route("/eth/v1/builder/status", get(get_status))
        .route("/eth/v1/builder/header/{slot}/{parent_hash}/{pubkey}", get(get_header::<I, A, E>))
        .with_state(api_impl)
}

async fn register_validators<I, A, E>(
    State(api_impl): State<I>,
    JsonOrSsz(registrations): JsonOrSsz<ValidatorRegistrations<E>>,
) -> Result<Response<Body>, StatusCode>
where
    E: EthSpec,
    I: AsRef<A> + Send + Sync,
    A: Builder<E>,
{
    let res = api_impl.as_ref().register_validators(registrations.to_vec()).await;
    build_response(res)
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

    build_response_with_headers(res, content_type, api_impl.as_ref().fork_name_at_slot(slot))
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
    let content_type_str =
        content_type_header.and_then(|value| value.to_str().ok()).unwrap_or("application/json");
    let content_type = match Accept::from_str(content_type_str) {
        Ok(Accept::Ssz) => ContentType::Ssz,
        _ => ContentType::Json,
    };

    let res = api_impl.as_ref().get_header(slot, parent_hash, pubkey).await;
    build_response_with_headers(res, content_type, api_impl.as_ref().fork_name_at_slot(slot))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::{body::Body, http::Request};
    use builder_api_types::{
        builder_bid::{BuilderBid, BuilderBidDeneb, SignedBuilderBid},
        Address, BeaconBlock, BeaconBlockDeneb, Blob, BlobsBundle, EmptyBlock, ExecutionPayload,
        ExecutionPayloadAndBlobs, ExecutionPayloadDeneb, ForkName, ForkVersionDecode,
        ForkVersionedResponse, FullPayloadContents, KzgCommitment, KzgProof, MainnetEthSpec,
        Signature, Uint256, ValidatorRegistrationData,
    };
    use ethereum_apis_common::{ErrorResponse, CONSENSUS_VERSION_HEADER};
    use http::{HeaderValue, Response};
    use ssz::Encode;
    use std::marker::PhantomData;
    use tower::ServiceExt;

    pub const PREFERENCE_ACCEPT_VALUE: &str =
        "application/octet-stream;q=1.0,application/json;q=0.9";

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
        fn fork_name_at_slot(&self, _slot: Slot) -> ForkName {
            ForkName::Deneb
        }

        async fn get_header(
            &self,
            _slot: Slot,
            _parent_hash: ExecutionBlockHash,
            _pubkey: PublicKeyBytes,
        ) -> Result<SignedBuilderBid<E>, ErrorResponse> {
            Ok(SignedBuilderBid {
                message: BuilderBid::Deneb(BuilderBidDeneb {
                    value: Uint256::from(42),
                    pubkey: PublicKeyBytes::empty(),
                    blob_kzg_commitments: vec![KzgCommitment::empty_for_testing(); 5].into(),
                    header: Default::default(),
                }),
                signature: Signature::empty(),
            })
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
                execution_payload: ExecutionPayload::Deneb(ExecutionPayloadDeneb::default()),
            };
            let full_payload = FullPayloadContents::PayloadAndBlobs(payload_and_blobs);
            Ok(full_payload)
        }
    }

    async fn send_request_and_assert_response(
        request: Request<Body>,
        expected_status: StatusCode,
        check_headers: impl AsyncFn(Response<Body>),
    ) {
        let app = new(DummyBuilder::<MainnetEthSpec> { _phantom: PhantomData });

        let response = app.oneshot(request).await.unwrap();
        // Assert status code
        assert_eq!(response.status(), expected_status);
        // Check headers
        check_headers(response).await;
    }

    #[tokio::test]
    async fn test_registration() {
        let dummy_registration: ValidatorRegistrations<MainnetEthSpec> = VariableList::from(vec![
            SignedValidatorRegistrationData {
                message: ValidatorRegistrationData {
                    fee_recipient: Address::random(),
                    gas_limit: 100000,
                    timestamp: 19939149139,
                    pubkey: PublicKeyBytes::empty(),
                },
                signature: Signature::empty()
            };
            100
        ]);

        // Test ssz request
        send_request_and_assert_response(
            Request::builder()
                .uri("/eth/v1/builder/validators")
                .method("POST")
                .header(CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"))
                .body(Body::from(dummy_registration.as_ssz_bytes()))
                .unwrap(),
            StatusCode::OK,
            async |_| {},
        )
        .await;

        // Test json request
        send_request_and_assert_response(
            Request::builder()
                .uri("/eth/v1/builder/validators")
                .method("POST")
                .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                .body(Body::from(serde_json::to_vec(&dummy_registration).unwrap()))
                .unwrap(),
            StatusCode::OK,
            async |_| {},
        )
        .await;
    }

    #[tokio::test]
    async fn test_get_header() {
        // Test ssz request
        send_request_and_assert_response(
            Request::builder()
                .uri(format!(
                    "/eth/v1/builder/header/{}/{}/{}",
                    Slot::new(42),
                    "0x379b447308533668e5323f45b7d5232259f508e8d61ff5b945c9b016792cd94c",
                    "0xafda62054797148859d1f277ad04e8129bc767c10dae0e2d116f03b87fe9c2a36093a93eab75b4b5bfd3fd0d48816396"
                ))
                .method("GET")
                .header(
                    ACCEPT,
                    HeaderValue::from_static("application/octet-stream"),
                ).body(Body::empty())
                .unwrap(),
            StatusCode::OK,
            async |response| {
                let headers = response.headers();
                assert_eq!(headers.get(CONTENT_TYPE).unwrap(), HeaderValue::from_str(&ContentType::Ssz.to_string()).unwrap());
                assert_eq!(headers.get(CONSENSUS_VERSION_HEADER).unwrap(), HeaderValue::from_str(&ForkName::Deneb.to_string()).unwrap());
            }
        )
        .await;

        // Test json request
        send_request_and_assert_response(
            Request::builder()
                .uri(format!(
                    "/eth/v1/builder/header/{}/{}/{}",
                    Slot::new(42),
                    "0x379b447308533668e5323f45b7d5232259f508e8d61ff5b945c9b016792cd94c",
                    "0xafda62054797148859d1f277ad04e8129bc767c10dae0e2d116f03b87fe9c2a36093a93eab75b4b5bfd3fd0d48816396"
                ))
                .method("GET")
                .header(
                    ACCEPT,
                    HeaderValue::from_static("application/json"),
                ).body(Body::empty())
                .unwrap(),
            StatusCode::OK,
            async |response| {
                let headers = response.headers();
                assert_eq!(headers.get(CONTENT_TYPE).unwrap(), HeaderValue::from_str(&ContentType::Json.to_string()).unwrap());
                assert_eq!(headers.get(CONSENSUS_VERSION_HEADER).unwrap(), HeaderValue::from_str(&ForkName::Deneb.to_string()).unwrap());
            }
        )
        .await;
    }

    #[tokio::test]
    async fn test_submit_blinded_beacon_block() {
        let spec = MainnetEthSpec::default_spec();
        let dummy_block = SignedBlindedBeaconBlock::<MainnetEthSpec>::from_block(
            BeaconBlock::Deneb(BeaconBlockDeneb::empty(&spec)),
            Signature::empty(),
        );
        // Test ssz request
        send_request_and_assert_response(
            Request::builder()
                .uri("/eth/v1/builder/blinded_blocks")
                .method("POST")
                .header(CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"))
                .header(ACCEPT, HeaderValue::from_static(PREFERENCE_ACCEPT_VALUE))
                .header(
                    CONSENSUS_VERSION_HEADER,
                    HeaderValue::from_str(&ForkName::Deneb.to_string()).unwrap(),
                )
                .body(Body::from(dummy_block.as_ssz_bytes()))
                .unwrap(),
            StatusCode::OK,
            async |response: Response<Body>| {
                let headers = response.headers();
                assert_eq!(
                    headers.get(CONTENT_TYPE).unwrap(),
                    HeaderValue::from_str(&ContentType::Ssz.to_string()).unwrap()
                );
                assert_eq!(
                    headers.get(CONSENSUS_VERSION_HEADER).unwrap(),
                    HeaderValue::from_str(&ForkName::Deneb.to_string()).unwrap()
                );

                // Get response body as bytes
                let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                    .await
                    .expect("should get bytes response");
                assert!(FullPayloadContents::<MainnetEthSpec>::from_ssz_bytes_by_fork(
                    &body,
                    ForkName::Deneb
                )
                .is_ok());
            },
        )
        .await;

        // Test json request
        send_request_and_assert_response(
            Request::builder()
                .uri("/eth/v1/builder/blinded_blocks")
                .method("POST")
                .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                .header(ACCEPT, HeaderValue::from_static("application/json"))
                .header(
                    CONSENSUS_VERSION_HEADER,
                    HeaderValue::from_str(&ForkName::Deneb.to_string()).unwrap(),
                )
                .body(Body::from(serde_json::to_vec(&dummy_block).unwrap()))
                .unwrap(),
            StatusCode::OK,
            async |response: Response<Body>| {
                let headers = response.headers();
                assert_eq!(
                    headers.get(CONTENT_TYPE).unwrap(),
                    HeaderValue::from_str(&ContentType::Json.to_string()).unwrap()
                );
                assert_eq!(
                    headers.get(CONSENSUS_VERSION_HEADER).unwrap(),
                    HeaderValue::from_str(&ForkName::Deneb.to_string()).unwrap()
                );

                // Get response body as bytes
                let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                    .await
                    .expect("should get bytes response");
                assert!(serde_json::from_slice::<
                    ForkVersionedResponse<FullPayloadContents::<MainnetEthSpec>>,
                >(&body)
                .is_ok());
            },
        )
        .await;
    }
}
