use async_trait::async_trait;
use relay_api_types::{
    ErrorResponse, SignedHeaderSubmission, SubmitBlockQueryParams, SubmitBlockRequest,
};
use types::eth_spec::EthSpec;

#[async_trait]
pub trait OptimisticV2<E: EthSpec> {
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
}
