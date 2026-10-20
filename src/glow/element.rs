//! Glow element types.

use crate::error::GlowError;

/// Parameter type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ParameterType {
    /// Null/unknown type
    #[default]
    Null = 0,
    /// Integer parameter
    Integer = 1,
    /// Real (floating point) parameter
    Real = 2,
    /// String parameter
    String = 3,
    /// Boolean parameter
    Boolean = 4,
    /// Trigger (action) parameter
    Trigger = 5,
    /// Enumerated parameter
    Enum = 6,
    /// Octet string (binary data)
    Octets = 7,
}

impl TryFrom<i64> for ParameterType {
    type Error = GlowError;

    fn try_from(value: i64) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(ParameterType::Null),
            1 => Ok(ParameterType::Integer),
            2 => Ok(ParameterType::Real),
            3 => Ok(ParameterType::String),
            4 => Ok(ParameterType::Boolean),
            5 => Ok(ParameterType::Trigger),
            6 => Ok(ParameterType::Enum),
            7 => Ok(ParameterType::Octets),
            _ => Err(GlowError::InvalidField {
                field: "ParameterType".to_string(),
                details: format!("unknown type value: {}", value),
            }),
        }
    }
}

/// Parameter access mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ParameterAccess {
    /// No access
    None = 0,
    /// Read-only access
    #[default]
    Read = 1,
    /// Write-only access
    Write = 2,
    /// Read-write access
    ReadWrite = 3,
}

impl TryFrom<i64> for ParameterAccess {
    type Error = GlowError;

    fn try_from(value: i64) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(ParameterAccess::None),
            1 => Ok(ParameterAccess::Read),
            2 => Ok(ParameterAccess::Write),
            3 => Ok(ParameterAccess::ReadWrite),
            _ => Err(GlowError::InvalidField {
                field: "ParameterAccess".to_string(),
                details: format!("unknown access value: {}", value),
            }),
        }
    }
}

/// Matrix type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum MatrixType {
    /// One-to-N matrix (one source to multiple targets)
    #[default]
    OneToN = 0,
    /// One-to-one matrix
    OneToOne = 1,
    /// N-to-N matrix (any source to any target)
    NToN = 2,
}

impl TryFrom<i64> for MatrixType {
    type Error = GlowError;

    fn try_from(value: i64) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(MatrixType::OneToN),
            1 => Ok(MatrixType::OneToOne),
            2 => Ok(MatrixType::NToN),
            _ => Err(GlowError::InvalidField {
                field: "MatrixType".to_string(),
                details: format!("unknown type value: {}", value),
            }),
        }
    }
}

/// Matrix addressing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum MatrixAddressingMode {
    /// Linear addressing (0 to count-1)
    #[default]
    Linear = 0,
    /// Non-linear addressing (sparse)
    NonLinear = 1,
}

impl TryFrom<i64> for MatrixAddressingMode {
    type Error = GlowError;

    fn try_from(value: i64) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(MatrixAddressingMode::Linear),
            1 => Ok(MatrixAddressingMode::NonLinear),
            _ => Err(GlowError::InvalidField {
                field: "MatrixAddressingMode".to_string(),
                details: format!("unknown mode value: {}", value),
            }),
        }
    }
}

/// Connection operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ConnectionOperation {
    /// Absolute connection (replace)
    #[default]
    Absolute = 0,
    /// Connect (add)
    Connect = 1,
    /// Disconnect (remove)
    Disconnect = 2,
}

impl TryFrom<i64> for ConnectionOperation {
    type Error = GlowError;

    fn try_from(value: i64) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(ConnectionOperation::Absolute),
            1 => Ok(ConnectionOperation::Connect),
            2 => Ok(ConnectionOperation::Disconnect),
            _ => Err(GlowError::InvalidField {
                field: "ConnectionOperation".to_string(),
                details: format!("unknown operation value: {}", value),
            }),
        }
    }
}

/// Connection disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ConnectionDisposition {
    /// Tally (feedback)
    #[default]
    Tally = 0,
    /// Modified (needs update)
    Modified = 1,
    /// Pending (operation in progress)
    Pending = 2,
    /// Locked (cannot modify)
    Locked = 3,
}

impl TryFrom<i64> for ConnectionDisposition {
    type Error = GlowError;

    fn try_from(value: i64) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(ConnectionDisposition::Tally),
            1 => Ok(ConnectionDisposition::Modified),
            2 => Ok(ConnectionDisposition::Pending),
            3 => Ok(ConnectionDisposition::Locked),
            _ => Err(GlowError::InvalidField {
                field: "ConnectionDisposition".to_string(),
                details: format!("unknown disposition value: {}", value),
            }),
        }
    }
}

/// A value that can be held by a parameter.
#[derive(Debug, Clone, PartialEq)]
pub enum EmberValue {
    /// Integer value
    Integer(i64),
    /// Real (floating point) value
    Real(f64),
    /// String value
    String(String),
    /// Boolean value
    Boolean(bool),
    /// Octet string (binary data)
    Octets(Vec<u8>),
    /// Null/empty value
    Null,
}

impl EmberValue {
    /// Get as integer if possible.
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            EmberValue::Integer(v) => Some(*v),
            EmberValue::Real(v) => Some(*v as i64),
            EmberValue::Boolean(v) => Some(if *v { 1 } else { 0 }),
            _ => None,
        }
    }

    /// Get as real if possible.
    pub fn as_real(&self) -> Option<f64> {
        match self {
            EmberValue::Integer(v) => Some(*v as f64),
            EmberValue::Real(v) => Some(*v),
            _ => None,
        }
    }

    /// Get as string if possible.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            EmberValue::String(v) => Some(v),
            _ => None,
        }
    }

    /// Get as boolean if possible.
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            EmberValue::Boolean(v) => Some(*v),
            EmberValue::Integer(v) => Some(*v != 0),
            _ => None,
        }
    }

    /// Check if this value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, EmberValue::Null)
    }

    /// Get the parameter type for this value.
    pub fn parameter_type(&self) -> ParameterType {
        match self {
            EmberValue::Integer(_) => ParameterType::Integer,
            EmberValue::Real(_) => ParameterType::Real,
            EmberValue::String(_) => ParameterType::String,
            EmberValue::Boolean(_) => ParameterType::Boolean,
            EmberValue::Octets(_) => ParameterType::Octets,
            EmberValue::Null => ParameterType::Null,
        }
    }
}

impl Default for EmberValue {
    fn default() -> Self {
        EmberValue::Null
    }
}

impl From<i64> for EmberValue {
    fn from(v: i64) -> Self {
        EmberValue::Integer(v)
    }
}

impl From<i32> for EmberValue {
    fn from(v: i32) -> Self {
        EmberValue::Integer(v as i64)
    }
}

impl From<f64> for EmberValue {
    fn from(v: f64) -> Self {
        EmberValue::Real(v)
    }
}

impl From<String> for EmberValue {
    fn from(v: String) -> Self {
        EmberValue::String(v)
    }
}

impl From<&str> for EmberValue {
    fn from(v: &str) -> Self {
        EmberValue::String(v.to_string())
    }
}

impl From<bool> for EmberValue {
    fn from(v: bool) -> Self {
        EmberValue::Boolean(v)
    }
}

impl From<Vec<u8>> for EmberValue {
    fn from(v: Vec<u8>) -> Self {
        EmberValue::Octets(v)
    }
}

impl std::fmt::Display for EmberValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmberValue::Integer(v) => write!(f, "{}", v),
            EmberValue::Real(v) => write!(f, "{}", v),
            EmberValue::String(v) => write!(f, "{}", v),
            EmberValue::Boolean(v) => write!(f, "{}", v),
            EmberValue::Octets(v) => write!(f, "{:?}", v),
            EmberValue::Null => write!(f, "null"),
        }
    }
}

/// Tuple item description (for function arguments/results).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TupleItemDescription {
    /// Item type
    pub item_type: ParameterType,
    /// Item name
    pub name: Option<String>,
}

impl TupleItemDescription {
    /// Create a new tuple item description.
    pub fn new(item_type: ParameterType, name: Option<String>) -> Self {
        TupleItemDescription { item_type, name }
    }
}

/// Stream descriptor for streamed parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamDescriptor {
    /// Stream format
    pub format: StreamFormat,
    /// Offset in stream
    pub offset: i32,
}

/// Stream format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum StreamFormat {
    /// Unsigned 8-bit integer
    #[default]
    UInt8 = 0,
    /// Unsigned 16-bit integer (big-endian)
    UInt16BigEndian = 2,
    /// Unsigned 16-bit integer (little-endian)
    UInt16LittleEndian = 3,
    /// Unsigned 32-bit integer (big-endian)
    UInt32BigEndian = 4,
    /// Unsigned 32-bit integer (little-endian)
    UInt32LittleEndian = 5,
    /// Unsigned 64-bit integer (big-endian)
    UInt64BigEndian = 6,
    /// Unsigned 64-bit integer (little-endian)
    UInt64LittleEndian = 7,
    /// Signed 8-bit integer
    Int8 = 8,
    /// Signed 16-bit integer (big-endian)
    Int16BigEndian = 10,
    /// Signed 16-bit integer (little-endian)
    Int16LittleEndian = 11,
    /// Signed 32-bit integer (big-endian)
    Int32BigEndian = 12,
    /// Signed 32-bit integer (little-endian)
    Int32LittleEndian = 13,
    /// Signed 64-bit integer (big-endian)
    Int64BigEndian = 14,
    /// Signed 64-bit integer (little-endian)
    Int64LittleEndian = 15,
    /// 32-bit float (big-endian)
    Float32BigEndian = 20,
    /// 32-bit float (little-endian)
    Float32LittleEndian = 21,
    /// 64-bit float (big-endian)
    Float64BigEndian = 22,
    /// 64-bit float (little-endian)
    Float64LittleEndian = 23,
}

impl TryFrom<i64> for StreamFormat {
    type Error = GlowError;

    fn try_from(value: i64) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(StreamFormat::UInt8),
            2 => Ok(StreamFormat::UInt16BigEndian),
            3 => Ok(StreamFormat::UInt16LittleEndian),
            4 => Ok(StreamFormat::UInt32BigEndian),
            5 => Ok(StreamFormat::UInt32LittleEndian),
            6 => Ok(StreamFormat::UInt64BigEndian),
            7 => Ok(StreamFormat::UInt64LittleEndian),
            8 => Ok(StreamFormat::Int8),
            10 => Ok(StreamFormat::Int16BigEndian),
            11 => Ok(StreamFormat::Int16LittleEndian),
            12 => Ok(StreamFormat::Int32BigEndian),
            13 => Ok(StreamFormat::Int32LittleEndian),
            14 => Ok(StreamFormat::Int64BigEndian),
            15 => Ok(StreamFormat::Int64LittleEndian),
            20 => Ok(StreamFormat::Float32BigEndian),
            21 => Ok(StreamFormat::Float32LittleEndian),
            22 => Ok(StreamFormat::Float64BigEndian),
            23 => Ok(StreamFormat::Float64LittleEndian),
            _ => Err(GlowError::InvalidField {
                field: "StreamFormat".to_string(),
                details: format!("unknown format value: {}", value),
            }),
        }
    }
}

/// String-integer entry for enumeration maps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringIntegerPair {
    /// String key
    pub key: String,
    /// Integer value
    pub value: i64,
}

impl StringIntegerPair {
    /// Create a new pair.
    pub fn new(key: String, value: i64) -> Self {
        StringIntegerPair { key, value }
    }
}

/// Matrix label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// Base path (relative OID)
    pub base_path: Vec<u32>,
    /// Description string
    pub description: String,
}

impl Label {
    /// Create a new label.
    pub fn new(base_path: Vec<u32>, description: String) -> Self {
        Label { base_path, description }
    }
}
