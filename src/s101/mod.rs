//! S101 framing protocol implementation.
//!
//! S101 is the transport layer protocol used by Ember+ for framing messages
//! over TCP connections. It provides:
//! - Message framing with start/end markers
//! - Byte escaping (byte stuffing)
//! - CRC-16 error detection
//! - Message type identification
//! - Keep-alive mechanism

mod frame;
mod codec;
mod crc;
mod constants;

pub use frame::{S101Frame, S101Message, MessageType, Command};
pub use codec::{S101Codec, S101Decoder, S101Encoder};
pub use constants::*;

use crate::error::{S101Error, Result};

/// S101 slot identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Slot {
    /// Default slot (used for Ember+ messages)
    Default = 0x00,
}

impl TryFrom<u8> for Slot {
    type Error = S101Error;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Slot::Default),
            _ => Err(S101Error::InvalidSlot(value)),
        }
    }
}

/// S101 protocol version.
pub const S101_VERSION: u8 = 0x01;

/// DTD type for Glow (Ember+ payload type).
pub const DTD_GLOW: u8 = 0x01;

/// Application bytes for Glow.
pub const APP_BYTES_GLOW: [u8; 2] = [0x01, 0x31]; // Version 1.49

/// Encode a payload into an S101 frame.
pub fn encode_frame(payload: &[u8]) -> Vec<u8> {
    S101Encoder::encode_ember_packet(payload)
}

/// Decode an S101 frame and extract the payload.
pub fn decode_frame(data: &[u8]) -> Result<Vec<u8>> {
    let frame = S101Frame::decode(data)?;
    Ok(frame.payload)
}
