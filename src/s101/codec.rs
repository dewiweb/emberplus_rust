//! S101 codec for encoding and decoding streams.

use bytes::{BufMut, BytesMut};
use crate::error::{S101Error, Result};
use super::constants::*;
use super::frame::{S101Frame, S101Message};
use super::crc;

/// S101 stream decoder.
#[derive(Debug, Default)]
pub struct S101Decoder {
    buffer: BytesMut,
    in_frame: bool,
    escape_next: bool,
    /// Queue of complete frames waiting to be decoded
    complete_frames: std::collections::VecDeque<BytesMut>,
}

impl S101Decoder {
    /// Create a new decoder.
    pub fn new() -> Self {
        S101Decoder {
            buffer: BytesMut::new(),
            in_frame: false,
            escape_next: false,
            complete_frames: std::collections::VecDeque::new(),
        }
    }

    /// Feed data to the decoder.
    pub fn feed(&mut self, data: &[u8]) {
        for &byte in data {
            self.process_byte(byte);
        }
    }

    /// Process a single byte.
    fn process_byte(&mut self, byte: u8) {
        if self.escape_next {
            self.buffer.put_u8(byte ^ ESCAPE_XOR);
            self.escape_next = false;
        } else if byte == BOF {
            // Start of frame
            self.buffer.clear();
            self.in_frame = true;
        } else if byte == EOF {
            if self.in_frame && !self.buffer.is_empty() {
                // End of frame - save complete frame to queue
                let complete = self.buffer.split();
                self.complete_frames.push_back(complete);
                self.in_frame = false;
            }
        } else if byte == ESCAPE_BYTE && self.in_frame {
            self.escape_next = true;
        } else if self.in_frame {
            self.buffer.put_u8(byte);
        }
    }

    /// Try to extract a complete frame.
    pub fn decode_frame(&mut self) -> Result<Option<S101Frame>> {
        // Get next complete frame from queue
        let frame_data = match self.complete_frames.pop_front() {
            Some(data) => data,
            None => return Ok(None),
        };

        // Verify CRC
        if frame_data.len() < 5 {
            // Too short for a valid frame
            return Err(S101Error::IncompleteFrame.into());
        }

        if !crc::verify_crc(&frame_data) {
            let len = frame_data.len();
            let received = u16::from_be_bytes([frame_data[len - 2], frame_data[len - 1]]);
            let calculated = crc::calculate_crc(&frame_data[..len - 2]);
            return Err(S101Error::CrcMismatch {
                expected: calculated,
                actual: received,
            }.into());
        }

        // Remove CRC from data
        let payload_len = frame_data.len() - 2;
        let data = &frame_data[..payload_len];
        
        if data.len() < 3 {
            return Err(S101Error::IncompleteFrame.into());
        }

        let slot = data[0];
        let message_type = super::frame::MessageType::try_from(data[1])?;
        let command = super::frame::Command::try_from(data[2])?;

        // Extract flags and payload based on command type
        let (flags, payload) = match command {
            super::frame::Command::KeepAliveRequest | super::frame::Command::KeepAliveResponse => {
                // Keep-alive: slot(1) + msg_type(1) + cmd(1) + version(1) = 4 bytes, no payload
                (super::frame::EmberFlags::default(), vec![])
            }
            super::frame::Command::Ember => {
                // Ember data: slot(1) + msg_type(1) + cmd(1) + version(1) + flags(1) + dtd(1) + app_count(1) + app_bytes + payload
                if data.len() < 7 {
                    return Err(S101Error::IncompleteFrame.into());
                }
                let flags = super::frame::EmberFlags::decode(data[4]);
                let app_bytes_count = data[6] as usize;
                let payload_start = 7 + app_bytes_count;
                if payload_start > data.len() {
                    return Err(S101Error::IncompleteFrame.into());
                }
                (flags, data[payload_start..].to_vec())
            }
        };

        Ok(Some(S101Frame {
            slot,
            message_type,
            command,
            flags,
            payload,
        }))
    }

    /// Decode and return a message if available.
    pub fn decode_message(&mut self) -> Result<Option<S101Message>> {
        if let Some(frame) = self.decode_frame()? {
            Ok(Some(frame.to_message()))
        } else {
            Ok(None)
        }
    }
}

/// S101 stream encoder.
#[derive(Debug, Default)]
pub struct S101Encoder;

impl S101Encoder {
    /// Create a new encoder.
    pub fn new() -> Self {
        S101Encoder
    }

    /// Encode an Ember+ packet.
    pub fn encode_ember_packet(payload: &[u8]) -> Vec<u8> {
        let frame = S101Frame::ember_data(payload.to_vec());
        frame.encode()
    }

    /// Encode a keep-alive request.
    pub fn encode_keepalive_request() -> Vec<u8> {
        S101Frame::keepalive_request().encode()
    }

    /// Encode a keep-alive response.
    pub fn encode_keepalive_response() -> Vec<u8> {
        S101Frame::keepalive_response().encode()
    }

    /// Encode a message.
    pub fn encode_message(message: &S101Message) -> Vec<u8> {
        match message {
            S101Message::EmberData { payload, .. } => {
                Self::encode_ember_packet(payload)
            }
            S101Message::KeepAliveRequest => Self::encode_keepalive_request(),
            S101Message::KeepAliveResponse => Self::encode_keepalive_response(),
        }
    }
}

/// Combined S101 codec for bidirectional communication.
pub struct S101Codec {
    /// Decoder for incoming data
    pub decoder: S101Decoder,
    /// Encoder for outgoing data
    pub encoder: S101Encoder,
}

impl S101Codec {
    /// Create a new codec.
    pub fn new() -> Self {
        S101Codec {
            decoder: S101Decoder::new(),
            encoder: S101Encoder::new(),
        }
    }

    /// Feed data to the decoder.
    pub fn feed(&mut self, data: &[u8]) {
        self.decoder.feed(data);
    }

    /// Try to decode a frame.
    pub fn decode_frame(&mut self) -> Result<Option<S101Frame>> {
        self.decoder.decode_frame()
    }

    /// Encode an Ember packet.
    pub fn encode_ember_packet(&self, payload: &[u8]) -> Vec<u8> {
        S101Encoder::encode_ember_packet(payload)
    }
}

impl Default for S101Codec {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoder_basic() {
        let mut decoder = S101Decoder::new();
        
        // Create a simple frame
        let frame = S101Frame::ember_data(vec![0x01, 0x02, 0x03]);
        let encoded = frame.encode();
        
        decoder.feed(&encoded);
        let decoded = decoder.decode_frame().unwrap().unwrap();
        
        assert_eq!(decoded.payload, vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_decoder_streaming() {
        let mut decoder = S101Decoder::new();
        
        let frame = S101Frame::ember_data(vec![0xAA, 0xBB, 0xCC]);
        let encoded = frame.encode();
        
        // Feed byte by byte
        for &byte in &encoded {
            decoder.feed(&[byte]);
        }
        
        let decoded = decoder.decode_frame().unwrap().unwrap();
        assert_eq!(decoded.payload, vec![0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn test_keepalive_roundtrip() {
        let mut decoder = S101Decoder::new();
        
        let encoded = S101Encoder::encode_keepalive_request();
        decoder.feed(&encoded);
        
        let message = decoder.decode_message().unwrap().unwrap();
        assert!(message.is_keepalive_request());
    }
}
