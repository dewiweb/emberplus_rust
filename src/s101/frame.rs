//! S101 frame structures.

use crate::error::{S101Error, Result};
use super::constants::*;
use super::crc;

/// S101 message types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    /// Ember message type (for both data and commands)
    Ember = 0x0E,
}

impl TryFrom<u8> for MessageType {
    type Error = S101Error;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0x0E => Ok(MessageType::Ember),
            _ => Err(S101Error::InvalidMessageType(value)),
        }
    }
}

/// S101 commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    /// Ember data packet
    Ember = 0x00,
    /// Keep-alive request
    KeepAliveRequest = 0x01,
    /// Keep-alive response
    KeepAliveResponse = 0x02,
}

impl TryFrom<u8> for Command {
    type Error = S101Error;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Command::Ember),
            0x01 => Ok(Command::KeepAliveRequest),
            0x02 => Ok(Command::KeepAliveResponse),
            _ => Err(S101Error::InvalidCommand(value)),
        }
    }
}

/// Flags for Ember messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EmberFlags {
    /// First packet of a multi-packet message
    pub first_packet: bool,
    /// Last packet of a multi-packet message
    pub last_packet: bool,
    /// Empty packet (used for keep-alive in some implementations)
    pub empty_packet: bool,
}

impl EmberFlags {
    /// Create flags for a single complete packet.
    pub fn single_packet() -> Self {
        EmberFlags {
            first_packet: true,
            last_packet: true,
            empty_packet: false,
        }
    }

    /// Encode flags to a byte.
    pub fn encode(&self) -> u8 {
        let mut flags = 0u8;
        if self.first_packet {
            flags |= 0x80;
        }
        if self.last_packet {
            flags |= 0x40;
        }
        if self.empty_packet {
            flags |= 0x20;
        }
        flags
    }

    /// Decode flags from a byte.
    pub fn decode(byte: u8) -> Self {
        EmberFlags {
            first_packet: (byte & 0x80) != 0,
            last_packet: (byte & 0x40) != 0,
            empty_packet: (byte & 0x20) != 0,
        }
    }
}

/// An S101 message (high-level representation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum S101Message {
    /// Ember+ data packet
    EmberData {
        /// DTD type (usually 0x01 for Glow)
        dtd_type: u8,
        /// Application-specific bytes
        app_bytes: Vec<u8>,
        /// Packet flags
        flags: EmberFlags,
        /// Payload data
        payload: Vec<u8>,
    },
    /// Keep-alive request
    KeepAliveRequest,
    /// Keep-alive response
    KeepAliveResponse,
}

impl S101Message {
    /// Create an Ember data message.
    pub fn ember_data(payload: Vec<u8>) -> Self {
        S101Message::EmberData {
            dtd_type: 0x01, // Glow
            app_bytes: vec![0x01, 0x31], // Version 1.49
            flags: EmberFlags::single_packet(),
            payload,
        }
    }

    /// Check if this is a keep-alive request.
    pub fn is_keepalive_request(&self) -> bool {
        matches!(self, S101Message::KeepAliveRequest)
    }

    /// Check if this is a keep-alive response.
    pub fn is_keepalive_response(&self) -> bool {
        matches!(self, S101Message::KeepAliveResponse)
    }

    /// Get the payload if this is an Ember data message.
    pub fn payload(&self) -> Option<&[u8]> {
        match self {
            S101Message::EmberData { payload, .. } => Some(payload),
            _ => None,
        }
    }
}

/// An S101 frame (raw wire format).
#[derive(Debug, Clone)]
pub struct S101Frame {
    /// Slot number (usually 0)
    pub slot: u8,
    /// Message type
    pub message_type: MessageType,
    /// Command
    pub command: Command,
    /// Ember flags (for multi-packet messages)
    pub flags: EmberFlags,
    /// Payload (after header, before CRC)
    pub payload: Vec<u8>,
}

impl S101Frame {
    /// Create a new frame for Ember data.
    pub fn ember_data(payload: Vec<u8>) -> Self {
        S101Frame {
            slot: 0,
            message_type: MessageType::Ember,
            command: Command::Ember,
            flags: EmberFlags::single_packet(),
            payload,
        }
    }

    /// Create a keep-alive request frame.
    pub fn keepalive_request() -> Self {
        S101Frame {
            slot: 0,
            message_type: MessageType::Ember,
            command: Command::KeepAliveRequest,
            flags: EmberFlags::default(),
            payload: vec![],
        }
    }

    /// Create a keep-alive response frame.
    pub fn keepalive_response() -> Self {
        S101Frame {
            slot: 0,
            message_type: MessageType::Ember,
            command: Command::KeepAliveResponse,
            flags: EmberFlags::default(),
            payload: vec![],
        }
    }

    /// Encode this frame to bytes (with framing and escaping).
    pub fn encode(&self) -> Vec<u8> {
        let mut output = Vec::new();
        output.push(BOF);
        
        // Build content (will be used for CRC calculation)
        let mut content = Vec::new();
        
        // Header: slot, message type, command
        content.push(self.slot);
        content.push(self.message_type as u8);
        content.push(self.command as u8);
        
        // For keep-alive, just add version
        // For Ember data, add version, flags, DTD, app bytes, then payload
        match self.command {
            Command::KeepAliveRequest | Command::KeepAliveResponse => {
                content.push(0x01); // Version
            }
            Command::Ember => {
                content.push(0x01); // Version
                content.push(0xC0); // Flags: first + last packet
                content.push(0x01); // DTD type: Glow
                content.push(0x02); // App bytes count
                content.push(0x1F); // Minor version: 31
                content.push(0x02); // Major version: 2
                content.extend(&self.payload);
            }
        }
        
        // Escape content and write to output
        for &byte in &content {
            if byte >= S101_INV {
                output.push(ESCAPE_BYTE);
                output.push(byte ^ ESCAPE_XOR);
            } else {
                output.push(byte);
            }
        }
        
        // Calculate CRC on the escaped content (excluding BOF)
        let crc = crc::calculate_crc_escaped(&output[1..]);
        let crc_lo = (crc & 0xFF) as u8;
        let crc_hi = ((crc >> 8) & 0xFF) as u8;
        
        // Escape and append CRC bytes
        if crc_lo >= S101_INV {
            output.push(ESCAPE_BYTE);
            output.push(crc_lo ^ ESCAPE_XOR);
        } else {
            output.push(crc_lo);
        }
        if crc_hi >= S101_INV {
            output.push(ESCAPE_BYTE);
            output.push(crc_hi ^ ESCAPE_XOR);
        } else {
            output.push(crc_hi);
        }
        
        output.push(EOF);
        output
    }

    /// Decode a frame from bytes (removing framing and escaping).
    pub fn decode(data: &[u8]) -> Result<Self> {
        if data.len() < 2 {
            return Err(S101Error::IncompleteFrame.into());
        }
        
        // Check frame markers
        if data[0] != BOF {
            return Err(S101Error::InvalidFrameMarker(data[0]).into());
        }
        if data[data.len() - 1] != EOF {
            return Err(S101Error::InvalidFrameMarker(data[data.len() - 1]).into());
        }
        
        // Remove escaping
        let inner_data = &data[1..data.len() - 1];
        let mut unescaped = Vec::with_capacity(inner_data.len());
        let mut i = 0;
        
        while i < inner_data.len() {
            if inner_data[i] == ESCAPE_BYTE {
                if i + 1 >= inner_data.len() {
                    return Err(S101Error::InvalidEscapeSequence.into());
                }
                unescaped.push(inner_data[i + 1] ^ ESCAPE_XOR);
                i += 2;
            } else {
                unescaped.push(inner_data[i]);
                i += 1;
            }
        }
        
        // Verify CRC
        if !crc::verify_crc(&unescaped) {
            let len = unescaped.len();
            if len >= 2 {
                let received = u16::from_be_bytes([unescaped[len - 2], unescaped[len - 1]]);
                let calculated = crc::calculate_crc(&unescaped[..len - 2]);
                return Err(S101Error::CrcMismatch {
                    expected: calculated,
                    actual: received,
                }.into());
            }
            return Err(S101Error::IncompleteFrame.into());
        }
        
        // Remove CRC from payload
        let unescaped = &unescaped[..unescaped.len() - 2];
        
        if unescaped.len() < 3 {
            return Err(S101Error::IncompleteFrame.into());
        }
        
        let slot = unescaped[SLOT_OFFSET];
        let message_type = MessageType::try_from(unescaped[MESSAGE_TYPE_OFFSET])?;
        let command = Command::try_from(unescaped[COMMAND_OFFSET])?;
        
        // Extract flags and payload based on command type
        let (flags, payload) = match command {
            Command::KeepAliveRequest | Command::KeepAliveResponse => {
                // Keep-alive: slot(1) + msg_type(1) + cmd(1) + version(1) = 4 bytes, no payload
                (EmberFlags::default(), vec![])
            }
            Command::Ember => {
                // Ember data: slot(1) + msg_type(1) + cmd(1) + version(1) + flags(1) + dtd(1) + app_count(1) + app_bytes + payload
                if unescaped.len() < 7 {
                    return Err(S101Error::IncompleteFrame.into());
                }
                let flags = EmberFlags::decode(unescaped[4]);
                let app_bytes_count = unescaped[6] as usize;
                let payload_start = 7 + app_bytes_count;
                if payload_start > unescaped.len() {
                    return Err(S101Error::IncompleteFrame.into());
                }
                (flags, unescaped[payload_start..].to_vec())
            }
        };
        
        Ok(S101Frame {
            slot,
            message_type,
            command,
            flags,
            payload,
        })
    }

    /// Convert to a high-level message.
    pub fn to_message(&self) -> S101Message {
        match self.command {
            Command::Ember => S101Message::EmberData {
                dtd_type: 0x01,
                app_bytes: vec![0x1F, 0x02],
                flags: self.flags,
                payload: self.payload.clone(),
            },
            Command::KeepAliveRequest => S101Message::KeepAliveRequest,
            Command::KeepAliveResponse => S101Message::KeepAliveResponse,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_roundtrip() {
        let frame = S101Frame::ember_data(vec![0x01, 0x02, 0x03, 0x04]);
        let encoded = frame.encode();
        let decoded = S101Frame::decode(&encoded).unwrap();
        
        assert_eq!(frame.slot, decoded.slot);
        assert_eq!(frame.message_type, decoded.message_type);
        assert_eq!(frame.command, decoded.command);
        assert_eq!(frame.payload, decoded.payload);
    }

    #[test]
    fn test_keepalive_frame() {
        let request = S101Frame::keepalive_request();
        let encoded = request.encode();
        let decoded = S101Frame::decode(&encoded).unwrap();
        
        assert_eq!(decoded.message_type, MessageType::Ember);
        assert_eq!(decoded.command, Command::KeepAliveRequest);
    }

    #[test]
    fn test_escape_sequences() {
        // Test with bytes that need escaping
        let frame = S101Frame::ember_data(vec![BOF, ESCAPE_BYTE, 0x00, 0x1F]);
        let encoded = frame.encode();
        let decoded = S101Frame::decode(&encoded).unwrap();
        
        assert_eq!(frame.payload, decoded.payload);
    }
}
