use serde::{Deserialize, Serialize};

pub mod beaver;
pub mod flashbots;
pub mod titan;

pub use beaver::*;
pub use flashbots::*;
pub use titan::*;

/// Universal bundle submission RPC type
///
/// This type represents what Lynx accepts from external order flow providers.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SendBundleRequest {
    /// Flashbots bundle
    Flashbots(FlashbotsBundle),
    /// Beaverbuild bundle
    Beaver(BeaverBundle),
    /// Titan Builder bundle
    Titan(TitanBundle),
}
