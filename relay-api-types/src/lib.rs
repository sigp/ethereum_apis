pub use beacon_api_types::{
    superstruct, Address, BlobsBundle, EthSpec, ExecutionBlockHash, ExecutionPayloadBellatrix,
    ExecutionPayloadCapella, ExecutionPayloadDeneb, ExecutionPayloadElectra,
    ExecutionPayloadHeaderBellatrix, ExecutionPayloadHeaderCapella, ExecutionPayloadHeaderDeneb,
    ExecutionPayloadHeaderElectra, MainnetEthSpec, MinimalEthSpec, PublicKeyBytes, Signature,
    SignedValidatorRegistrationData, Slot, Uint256,
};
use serde::{Deserialize, Serialize};
use serde_utils::quoted_u64::Quoted;
use ssz_derive::{Decode, Encode};

// Builder API requests

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubmitBlockQueryParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancellations: Option<bool>,
}

#[superstruct(
    variants(Bellatrix, Capella, Deneb, Electra),
    variant_attributes(
        derive(Debug, Clone, Serialize, Deserialize, Encode, Decode),
        serde(bound = "E: EthSpec", deny_unknown_fields),
    ),
    map_into(ExecutionPayload),
    map_ref_into(ExecutionPayload)
)]
#[derive(Debug, Clone, Serialize, Deserialize, Encode)]
#[serde(bound = "E: EthSpec", untagged)]
#[ssz(enum_behaviour = "transparent")]
pub struct SubmitBlockRequest<E: EthSpec> {
    message: BidTraceV1,
    #[superstruct(flatten)]
    execution_payload: ExecutionPayload<E>,
    signature: Signature,
    #[superstruct(only(Deneb))]
    blobs_bundle: BlobsBundle<E>,
}

impl<E: EthSpec> ssz::Decode for SubmitBlockRequest<E> {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    // No Eth-Consensus-Types specified https://github.com/flashbots/relay-specs/issues/36
    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        let Ok(req) = SubmitBlockRequestElectra::from_ssz_bytes(bytes) else {
            let Ok(req) = SubmitBlockRequestDeneb::from_ssz_bytes(bytes) else {
                let Ok(req) = SubmitBlockRequestCapella::from_ssz_bytes(bytes) else {
                    return Ok(Self::Bellatrix(
                        SubmitBlockRequestBellatrix::from_ssz_bytes(bytes)?,
                    ));
                };
                return Ok(Self::Capella(req));
            };
            return Ok(Self::Deneb(req));
        };
        Ok(Self::Electra(req))
    }
}

// Data API requests

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderBy {
    #[serde(rename = "value")]
    Value,
    #[serde(rename = "-value")]
    NegativeValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetDeliveredPayloadsQueryParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot: Option<Slot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Slot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<Slot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<ExecutionBlockHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number: Option<Quoted<u64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposer_pubkey: Option<PublicKeyBytes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub builder_pubkey: Option<PublicKeyBytes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_by: Option<OrderBy>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetReceivedBidsQueryParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot: Option<Slot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<ExecutionBlockHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number: Option<Quoted<u64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub builder_pubkey: Option<PublicKeyBytes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<Slot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetValidatorRegistrationQueryParams {
    pub pubkey: PublicKeyBytes,
}

#[superstruct(
    variants(Bellatrix, Capella, Deneb, Electra),
    variant_attributes(
        derive(Debug, Clone, Serialize, Deserialize, Encode, Decode),
        serde(bound = "E: EthSpec", deny_unknown_fields),
    ),
    map_into(ExecutionPayloadHeader),
    map_ref_into(ExecutionPayloadHeader)
)]
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(bound = "E: EthSpec", untagged)]
#[ssz(enum_behaviour = "transparent")]
pub struct HeaderSubmission<E: EthSpec> {
    bid_trace: BidTraceV1,
    #[superstruct(flatten)]
    execution_payload_header: ExecutionPayloadHeader<E>,
    #[superstruct(only(Deneb))]
    blobs_bundle: BlobsBundle<E>,
}

#[superstruct(
    variants(Bellatrix, Capella, Deneb, Electra),
    variant_attributes(
        derive(Debug, Clone, Serialize, Deserialize, Encode, Decode),
        serde(bound = "E: EthSpec", deny_unknown_fields),
    ),
    map_into(ExecutionPayloadHeader),
    map_ref_into(ExecutionPayloadHeader)
)]
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(bound = "E: EthSpec", untagged)]
#[ssz(enum_behaviour = "transparent")]
pub struct SignedHeaderSubmission<E: EthSpec> {
    #[superstruct(flatten)]
    message: HeaderSubmission<E>,
    signature: Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Cancellation {
    #[serde(with = "serde_utils::quoted_u64")]
    pub slot: u64,
    pub parent_hash: ExecutionBlockHash,
    pub proposer_public_key: PublicKeyBytes,
    pub builder_public_key: PublicKeyBytes,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct SignedCancellation {
    pub message: Cancellation,
    pub signature: Signature,
}

// Websockets types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopBidUpdate {
    #[serde(with = "serde_utils::quoted_u64")]
    timestamp: u64,
    slot: Slot,
    #[serde(with = "serde_utils::quoted_u64")]
    block_number: u64,
    block_hash: ExecutionBlockHash,
    parent_hash: ExecutionBlockHash,
    builder_pubkey: PublicKeyBytes,
    fee_recipient: Address,
    #[serde(with = "serde_utils::quoted_u256")]
    value: Uint256,
}

// Builder API responses
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Filtering {
    Regional,
    Global,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidatorPreferences {
    filtering: Filtering,
    trusted_builders: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidatorsResponse {
    pub slot: Slot,
    #[serde(with = "serde_utils::quoted_u64")]
    pub validator_index: u64,
    pub entry: SignedValidatorRegistrationData,
    pub preferences: Option<ValidatorPreferences>,
}

// Data API responses

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct BidTraceV1 {
    pub slot: Slot,
    pub parent_hash: ExecutionBlockHash,
    pub block_hash: ExecutionBlockHash,
    pub builder_pubkey: PublicKeyBytes,
    pub proposer_pubkey: PublicKeyBytes,
    pub proposer_fee_recipient: Address,
    #[serde(with = "serde_utils::quoted_u64")]
    pub gas_limit: u64,
    #[serde(with = "serde_utils::quoted_u64")]
    pub gas_used: u64,
    #[serde(with = "serde_utils::quoted_u256")]
    pub value: Uint256,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BidTraceV2 {
    #[serde(flatten)]
    pub bid_trace: BidTraceV1,
    #[serde(with = "serde_utils::quoted_u64")]
    pub block_number: u64,
    #[serde(with = "serde_utils::quoted_u64")]
    pub num_tx: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BidTraceV2WithTimestamp {
    #[serde(flatten)]
    pub bid_trace: BidTraceV2,
    #[serde(with = "serde_utils::quoted_i64")]
    pub timestamp: i64,
    #[serde(with = "serde_utils::quoted_i64")]
    pub timestamp_ms: i64,
}

// Builder API response types
pub type GetValidatorsResponse = Vec<ValidatorsResponse>;

// Data API response types
pub type GetDeliveredPayloadsResponse = Vec<BidTraceV2>;
pub type GetReceivedBidsResponse = Vec<BidTraceV2WithTimestamp>;
pub type GetValidatorRegistrationResponse = SignedValidatorRegistrationData;
