use crate::{builder::Builder, data::Data};
use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::{
    body::Body,
    extract::{Query, State},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use ethereum_apis_common::{build_response, JsonOrSszMaybeGzipped};
use futures::{sink::SinkExt, stream::StreamExt};
use http::StatusCode;
use relay_api_types::{
    EthSpec, GetDeliveredPayloadsQueryParams, GetReceivedBidsQueryParams,
    GetValidatorRegistrationQueryParams, SignedCancellation, SignedHeaderSubmission,
    SubmitBlockQueryParams, SubmitBlockRequest,
};
use std::net::SocketAddr;

/// Setup API Server.
pub fn new<I, A, E>(api_impl: I) -> Router
where
    E: EthSpec,
    I: AsRef<A> + Clone + Send + Sync + 'static,
    A: Builder<E> + Data + 'static,
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
        .route("/relay/v1/builder/headers", post(submit_header::<I, A, E>))
        .route(
            "/relay/v1/builder/cancel_bid",
            post(submit_cancellation::<I, A, E>),
        )
        .route("/relay/v1/builder/top_bids", get(get_top_bids::<I, A, E>))
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
        .with_state(api_impl)
}

/// SubmitBlock - POST /relay/v1/builder/blocks
#[tracing::instrument(skip_all)]
async fn submit_block<I, A, E>(
    Query(query_params): Query<SubmitBlockQueryParams>,
    State(api_impl): State<I>,
    JsonOrSszMaybeGzipped(body): JsonOrSszMaybeGzipped<SubmitBlockRequest<E>>,
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
    JsonOrSszMaybeGzipped(body): JsonOrSszMaybeGzipped<SubmitBlockRequest<E>>,
) -> Result<Response<Body>, StatusCode>
where
    E: EthSpec,
    I: AsRef<A> + Send + Sync,
    A: Builder<E>,
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
    JsonOrSszMaybeGzipped(body): JsonOrSszMaybeGzipped<SignedHeaderSubmission<E>>,
) -> Result<Response<Body>, StatusCode>
where
    E: EthSpec,
    I: AsRef<A> + Send + Sync,
    A: Builder<E>,
{
    let result = api_impl.as_ref().submit_header(query_params, body).await;
    build_response(result).await
}

/// SubmitCancellation - POST /relay/v1/builder/cancel_bid
#[tracing::instrument(skip_all)]
async fn submit_cancellation<I, A, E>(
    State(api_impl): State<I>,
    JsonOrSszMaybeGzipped(body): JsonOrSszMaybeGzipped<SignedCancellation>,
) -> Result<Response<Body>, StatusCode>
where
    E: EthSpec,
    I: AsRef<A> + Send + Sync,
    A: Builder<E>,
{
    let result = api_impl.as_ref().submit_cancellation(body).await;
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

/// GetTopBids - GET /relay/v1/builder/top_bids
#[tracing::instrument(skip_all)]
async fn get_top_bids<I, A, E>(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(api_impl): State<I>,
) -> impl IntoResponse
where
    I: AsRef<A> + Send + Sync + 'static,
    A: Builder<E> + 'static,
    E: EthSpec,
{
    ws.on_upgrade(move |socket| handle_socket(socket, addr, api_impl))
}

async fn handle_socket<I, A, E>(socket: WebSocket, who: SocketAddr, api_impl: I)
where
    I: AsRef<A> + Send + Sync + 'static,
    A: Builder<E>,
    E: EthSpec,
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
            if let Message::Close(_) = message {
                break;
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    tracing::info!("Client {} disconnected", who);
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
