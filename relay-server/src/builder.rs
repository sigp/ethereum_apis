use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use relay_api_types::{
    ErrorResponse, GetValidatorsResponse, SignedCancellation, SignedHeaderSubmission,
    SubmitBlockQueryParams, SubmitBlockRequest, TopBidUpdate,
};
use types::eth_spec::EthSpec;

/// Builder
#[async_trait]
pub trait Builder<E: EthSpec> {
    /// Get a list of validator registrations for validators scheduled to propose in the current and next epoch. .
    ///
    /// GetValidators - GET /relay/v1/builder/validators
    async fn get_validators(&self) -> Result<GetValidatorsResponse, ErrorResponse>;

    /// Submit a new block to the relay..
    ///
    /// SubmitBlock - POST /relay/v1/builder/blocks
    async fn submit_block(
        &self,
        query_params: SubmitBlockQueryParams,
        body: SubmitBlockRequest<E>,
    ) -> Result<(), ErrorResponse>;
    /// Submit a new block header to the relay.
    ///
    /// SubmitHeader - POST /relay/v1/builder/headers
    async fn submit_header(
        &self,
        query_params: SubmitBlockQueryParams,
        body: SignedHeaderSubmission<E>,
    ) -> Result<(), ErrorResponse>;

    /// Submit a new block to the relay optimistically.
    ///
    /// SubmitBlockOptimisticV2 - POST /relay/v1/builder/blocks_optimistic_v2
    async fn submit_block_optimistic_v2(
        &self,
        query_params: SubmitBlockQueryParams,
        body: SubmitBlockRequest<E>,
    ) -> Result<(), ErrorResponse>;

    /// Submit a cancellation for all bids.
    ///
    /// SubmitCancellation- POST /relay/v1/builder/cancel_bid
    async fn submit_cancellation(&self, body: SignedCancellation) -> Result<(), ErrorResponse>;

    /// Open a WebSockets stream of top bids from the relay.
    ///
    /// GetTopBids - GET /relay/v1/builder/top_bid
    async fn get_top_bids(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = TopBidUpdate> + Send>>, ErrorResponse>;
}
