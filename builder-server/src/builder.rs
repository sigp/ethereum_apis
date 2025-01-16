use async_trait::async_trait;
use builder_api_types::{
    builder_bid::SignedBuilderBid, eth_spec::EthSpec, ExecutionBlockHash, ForkName,
    FullPayloadContents, PublicKeyBytes, SignedBlindedBeaconBlock, SignedValidatorRegistrationData,
    Slot,
};
use ethereum_apis_common::ErrorResponse;

#[async_trait]
pub trait Builder<E: EthSpec> {
    async fn register_validators(
        &self,
        registrations: Vec<SignedValidatorRegistrationData>,
    ) -> Result<(), ErrorResponse>;

    async fn submit_blinded_block(
        &self,
        block: SignedBlindedBeaconBlock<E>,
    ) -> Result<FullPayloadContents<E>, ErrorResponse>;

    async fn get_header(
        &self,
        slot: Slot,
        parent_hash: ExecutionBlockHash,
        pubkey: PublicKeyBytes,
    ) -> Result<SignedBuilderBid<E>, ErrorResponse>;

    fn fork_name_at_slot(&self, slot: Slot) -> ForkName;
}
