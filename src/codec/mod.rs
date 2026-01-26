//! Glow codec for encoding and decoding Ember+ messages.

mod encoder;
mod decoder;

pub use encoder::GlowEncoder;
pub use decoder::GlowDecoder;

use crate::error::Result;
use crate::glow::GlowRoot;

/// Encode a GlowRoot to BER bytes.
pub fn encode(root: &GlowRoot) -> Result<Vec<u8>> {
    GlowEncoder::encode(root)
}

/// Decode BER bytes to a GlowRoot.
pub fn decode(data: &[u8]) -> Result<GlowRoot> {
    GlowDecoder::decode(data)
}
