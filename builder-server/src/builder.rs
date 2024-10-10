use async_trait::async_trait;
use ethereum_apis_common::ErrorResponse;
use types::{
    builder_bid::SignedBuilderBid, eth_spec::EthSpec, ExecutionBlockHash, ExecutionPayload,
    ForkName, PublicKeyBytes, SignedBlindedBeaconBlock, SignedValidatorRegistrationData, Slot,
};

#[async_trait]
pub trait Builder<E: EthSpec> {
    async fn register_validators(
        &self,
        registrations: Vec<SignedValidatorRegistrationData>,
    ) -> Result<(), ErrorResponse>;

    async fn submit_blinded_block(
        &self,
        block: SignedBlindedBeaconBlock<E>,
    ) -> Result<ExecutionPayload<E>, ErrorResponse>;

    async fn get_header(
        &self,
        slot: Slot,
        parent_hash: ExecutionBlockHash,
        pubkey: PublicKeyBytes,
    ) -> Result<SignedBuilderBid<E>, ErrorResponse>;

    fn fork_name_at_slot(&self, slot: Slot) -> ForkName;
}
