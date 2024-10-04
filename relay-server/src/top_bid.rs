use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use relay_api_types::{ErrorResponse, TopBidUpdate};

/// TopBids
#[async_trait]
pub trait TopBids {
    /// Open a WebSockets stream of top bids from the relay.
    ///
    /// GetTopBids - GET /relay/v1/builder/top_bid
    async fn get_top_bids(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = TopBidUpdate> + Send>>, ErrorResponse>;
}
