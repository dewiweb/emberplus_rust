//! S101 protocol constants.

/// Frame start marker (BOF).
pub const BOF: u8 = 0xFE;

/// Frame end marker (EOF).
pub const EOF: u8 = 0xFF;

/// Escape byte for byte stuffing.
pub const ESCAPE_BYTE: u8 = 0xFD;

/// XOR value for escaped bytes.
pub const ESCAPE_XOR: u8 = 0x20;

/// Invalid byte that must be escaped.
pub const INVALID_BYTE: u8 = 0xFC;

/// Slot byte position.
pub const SLOT_OFFSET: usize = 0;

/// Message type byte position.
pub const MESSAGE_TYPE_OFFSET: usize = 1;

/// Command byte position.
pub const COMMAND_OFFSET: usize = 2;

/// Version byte position (for Ember messages).
pub const VERSION_OFFSET: usize = 3;

/// Maximum frame size.
pub const MAX_FRAME_SIZE: usize = 65535;

/// Keep-alive request command.
pub const CMD_KEEPALIVE_REQUEST: u8 = 0x01;

/// Keep-alive response command.
pub const CMD_KEEPALIVE_RESPONSE: u8 = 0x02;

/// Ember data command.
pub const CMD_EMBER: u8 = 0x00;

/// Message type: Ember (for both data and keep-alive commands).
pub const MSG_TYPE_EMBER: u8 = 0x0E;

/// Invalid byte threshold - bytes >= this must be escaped.
pub const S101_INV: u8 = 0xF8;

/// CRC polynomial (CRC-16-CCITT).
pub const CRC_POLYNOMIAL: u16 = 0x1021;

/// CRC initial value.
pub const CRC_INITIAL: u16 = 0xFFFF;

/// Header length for Ember messages.
pub const EMBER_HEADER_LEN: usize = 7;
