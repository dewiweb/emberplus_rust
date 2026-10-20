//! Error types for the Ember+ library.

use thiserror::Error;

/// Result type alias for Ember+ operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Ember+ error types.
#[derive(Debug, Error)]
pub enum Error {
    /// I/O error during network operations.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// UTF-8 conversion error.
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    /// BER encoding/decoding error.
    #[error("BER codec error: {0}")]
    Ber(#[from] BerError),

    /// S101 framing error.
    #[error("S101 framing error: {0}")]
    S101(#[from] S101Error),

    /// Glow protocol error.
    #[error("Glow protocol error: {0}")]
    Glow(#[from] GlowError),

    /// Connection error.
    #[error("Connection error: {0}")]
    Connection(String),

    /// Timeout error.
    #[error("Operation timed out")]
    Timeout,

    /// Path not found in tree.
    #[error("Path not found: {0}")]
    PathNotFound(String),

    /// Invalid operation.
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    /// Protocol version mismatch.
    #[error("Protocol version mismatch: expected {expected}, got {actual}")]
    VersionMismatch {
        /// Expected protocol version
        expected: u8,
        /// Actual protocol version received
        actual: u8,
    },

    /// Server rejected the request.
    #[error("Server rejected request: {0}")]
    Rejected(String),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// BER encoding/decoding errors.
#[derive(Debug, Error)]
pub enum BerError {
    /// Unexpected end of input.
    #[error("Unexpected end of input")]
    UnexpectedEof,

    /// Invalid tag encoding.
    #[error("Invalid tag: {0}")]
    InvalidTag(String),

    /// Invalid length encoding.
    #[error("Invalid length encoding")]
    InvalidLength,

    /// Length overflow.
    #[error("Length overflow: value too large")]
    LengthOverflow,

    /// Invalid value encoding.
    #[error("Invalid value encoding for type {type_name}: {details}")]
    InvalidValue {
        /// Name of the type being decoded
        type_name: String,
        /// Details about the invalid value
        details: String,
    },

    /// Unsupported tag class.
    #[error("Unsupported tag class: {0}")]
    UnsupportedTagClass(u8),

    /// Buffer too small.
    #[error("Buffer too small: need {needed}, have {available}")]
    BufferTooSmall {
        /// Number of bytes needed
        needed: usize,
        /// Number of bytes available
        available: usize,
    },

    /// Invalid UTF-8 string.
    #[error("Invalid UTF-8 string: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),

    /// Integer conversion error.
    #[error("Integer conversion error")]
    IntegerConversion,
}

/// S101 framing errors.
#[derive(Debug, Error)]
pub enum S101Error {
    /// Invalid frame marker.
    #[error("Invalid frame marker: expected 0xFE, got 0x{0:02X}")]
    InvalidFrameMarker(u8),

    /// Invalid slot number.
    #[error("Invalid slot number: {0}")]
    InvalidSlot(u8),

    /// Invalid message type.
    #[error("Invalid message type: 0x{0:02X}")]
    InvalidMessageType(u8),

    /// Invalid command.
    #[error("Invalid command: 0x{0:02X}")]
    InvalidCommand(u8),

    /// CRC mismatch.
    #[error("CRC mismatch: expected 0x{expected:04X}, got 0x{actual:04X}")]
    CrcMismatch {
        /// Expected CRC value
        expected: u16,
        /// Computed CRC value
        actual: u16,
    },

    /// Frame too large.
    #[error("Frame too large: {size} bytes exceeds maximum {max}")]
    FrameTooLarge {
        /// Size of the received frame
        size: usize,
        /// Maximum allowed frame size
        max: usize,
    },

    /// Incomplete frame.
    #[error("Incomplete frame: need more data")]
    IncompleteFrame,

    /// Invalid escape sequence.
    #[error("Invalid escape sequence")]
    InvalidEscapeSequence,
}

/// Glow protocol errors.
#[derive(Debug, Error)]
pub enum GlowError {
    /// Unknown element type.
    #[error("Unknown element type: {0}")]
    UnknownElementType(u8),

    /// Missing required field.
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Invalid field value.
    #[error("Invalid field value for {field}: {details}")]
    InvalidField {
        /// Name of the invalid field
        field: String,
        /// Details about the invalid value
        details: String,
    },

    /// Invalid tree structure.
    #[error("Invalid tree structure: {0}")]
    InvalidTree(String),

    /// Unsupported feature.
    #[error("Unsupported feature: {0}")]
    Unsupported(String),

    /// Invalid path format.
    #[error("Invalid path format: {0}")]
    InvalidPath(String),

    /// Type mismatch.
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        /// Expected type
        expected: String,
        /// Actual type encountered
        actual: String,
    },

    /// Invalid matrix operation.
    #[error("Invalid matrix operation: {0}")]
    InvalidMatrixOperation(String),

    /// Invocation failed.
    #[error("Function invocation failed: {0}")]
    InvocationFailed(String),
}

impl Error {
    /// Create a connection error.
    pub fn connection<S: Into<String>>(msg: S) -> Self {
        Error::Connection(msg.into())
    }

    /// Create an invalid operation error.
    pub fn invalid_operation<S: Into<String>>(msg: S) -> Self {
        Error::InvalidOperation(msg.into())
    }

    /// Create a path not found error.
    pub fn path_not_found<S: Into<String>>(path: S) -> Self {
        Error::PathNotFound(path.into())
    }

    /// Create an internal error.
    pub fn internal<S: Into<String>>(msg: S) -> Self {
        Error::Internal(msg.into())
    }
}

impl BerError {
    /// Create an invalid value error.
    pub fn invalid_value<S1: Into<String>, S2: Into<String>>(type_name: S1, details: S2) -> Self {
        BerError::InvalidValue {
            type_name: type_name.into(),
            details: details.into(),
        }
    }
}

impl GlowError {
    /// Create a missing field error.
    pub fn missing_field<S: Into<String>>(field: S) -> Self {
        GlowError::MissingField(field.into())
    }

    /// Create an invalid field error.
    pub fn invalid_field<S1: Into<String>, S2: Into<String>>(field: S1, details: S2) -> Self {
        GlowError::InvalidField {
            field: field.into(),
            details: details.into(),
        }
    }
}
