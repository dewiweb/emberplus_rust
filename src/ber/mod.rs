//! BER (Basic Encoding Rules) codec for ASN.1.
//!
//! This module implements the ASN.1 BER encoding used by Ember+ protocol.
//! The Ember+ protocol uses BER encoding for all data serialization.

mod tag;
mod length;
mod reader;
mod writer;
mod value;

pub use tag::{Tag, TagClass, TagType};
pub use length::Length;
pub use reader::BerReader;
pub use writer::BerWriter;
pub use value::BerValue;

use crate::error::{BerError, Result};

/// Universal tag numbers for ASN.1 primitive types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UniversalTag {
    /// Boolean type (tag 1)
    Boolean = 1,
    /// Integer type (tag 2)
    Integer = 2,
    /// Bit string type (tag 3)
    BitString = 3,
    /// Octet string type (tag 4)
    OctetString = 4,
    /// Null type (tag 5)
    Null = 5,
    /// Object identifier type (tag 6)
    ObjectIdentifier = 6,
    /// Object descriptor type (tag 7)
    ObjectDescriptor = 7,
    /// External type (tag 8)
    External = 8,
    /// Real type (tag 9)
    Real = 9,
    /// Enumerated type (tag 10)
    Enumerated = 10,
    /// Embedded PDV type (tag 11)
    EmbeddedPdv = 11,
    /// UTF-8 string type (tag 12)
    Utf8String = 12,
    /// Relative OID type (tag 13)
    RelativeOid = 13,
    /// Sequence type (tag 16)
    Sequence = 16,
    /// Set type (tag 17)
    Set = 17,
    /// Numeric string type (tag 18)
    NumericString = 18,
    /// Printable string type (tag 19)
    PrintableString = 19,
    /// T61 string type (tag 20)
    T61String = 20,
    /// Videotex string type (tag 21)
    VideotexString = 21,
    /// IA5 string type (tag 22)
    Ia5String = 22,
    /// UTC time type (tag 23)
    UtcTime = 23,
    /// Generalized time type (tag 24)
    GeneralizedTime = 24,
    /// Graphic string type (tag 25)
    GraphicString = 25,
    /// Visible string type (tag 26)
    VisibleString = 26,
    /// General string type (tag 27)
    GeneralString = 27,
    /// Universal string type (tag 28)
    UniversalString = 28,
    /// Character string type (tag 29)
    CharacterString = 29,
    /// BMP string type (tag 30)
    BmpString = 30,
}

impl TryFrom<u32> for UniversalTag {
    type Error = BerError;

    fn try_from(value: u32) -> std::result::Result<Self, Self::Error> {
        match value {
            1 => Ok(UniversalTag::Boolean),
            2 => Ok(UniversalTag::Integer),
            3 => Ok(UniversalTag::BitString),
            4 => Ok(UniversalTag::OctetString),
            5 => Ok(UniversalTag::Null),
            6 => Ok(UniversalTag::ObjectIdentifier),
            7 => Ok(UniversalTag::ObjectDescriptor),
            8 => Ok(UniversalTag::External),
            9 => Ok(UniversalTag::Real),
            10 => Ok(UniversalTag::Enumerated),
            11 => Ok(UniversalTag::EmbeddedPdv),
            12 => Ok(UniversalTag::Utf8String),
            13 => Ok(UniversalTag::RelativeOid),
            16 => Ok(UniversalTag::Sequence),
            17 => Ok(UniversalTag::Set),
            18 => Ok(UniversalTag::NumericString),
            19 => Ok(UniversalTag::PrintableString),
            20 => Ok(UniversalTag::T61String),
            21 => Ok(UniversalTag::VideotexString),
            22 => Ok(UniversalTag::Ia5String),
            23 => Ok(UniversalTag::UtcTime),
            24 => Ok(UniversalTag::GeneralizedTime),
            25 => Ok(UniversalTag::GraphicString),
            26 => Ok(UniversalTag::VisibleString),
            27 => Ok(UniversalTag::GeneralString),
            28 => Ok(UniversalTag::UniversalString),
            29 => Ok(UniversalTag::CharacterString),
            30 => Ok(UniversalTag::BmpString),
            _ => Err(BerError::InvalidTag(format!("unknown universal tag: {}", value))),
        }
    }
}

/// Encode a value to BER format.
pub fn encode<T: BerEncode>(value: &T) -> Result<Vec<u8>> {
    let mut writer = BerWriter::new();
    value.encode(&mut writer)?;
    Ok(writer.into_bytes())
}

/// Decode a value from BER format.
pub fn decode<T: BerDecode>(data: &[u8]) -> Result<T> {
    let mut reader = BerReader::new(data);
    T::decode(&mut reader)
}

/// Trait for types that can be encoded to BER format.
pub trait BerEncode {
    /// Encode this value to the writer.
    fn encode(&self, writer: &mut BerWriter) -> Result<()>;
}

/// Trait for types that can be decoded from BER format.
pub trait BerDecode: Sized {
    /// Decode a value from the reader.
    fn decode(reader: &mut BerReader) -> Result<Self>;
}

// Implement BerEncode for primitive types
impl BerEncode for bool {
    fn encode(&self, writer: &mut BerWriter) -> Result<()> {
        writer.write_boolean(*self)
    }
}

impl BerEncode for i64 {
    fn encode(&self, writer: &mut BerWriter) -> Result<()> {
        writer.write_integer(*self)
    }
}

impl BerEncode for i32 {
    fn encode(&self, writer: &mut BerWriter) -> Result<()> {
        writer.write_integer(*self as i64)
    }
}

impl BerEncode for f64 {
    fn encode(&self, writer: &mut BerWriter) -> Result<()> {
        writer.write_real(*self)
    }
}

impl BerEncode for str {
    fn encode(&self, writer: &mut BerWriter) -> Result<()> {
        writer.write_utf8_string(self)
    }
}

impl BerEncode for String {
    fn encode(&self, writer: &mut BerWriter) -> Result<()> {
        writer.write_utf8_string(self)
    }
}

impl BerEncode for [u8] {
    fn encode(&self, writer: &mut BerWriter) -> Result<()> {
        writer.write_octet_string(self)
    }
}

impl BerEncode for Vec<u8> {
    fn encode(&self, writer: &mut BerWriter) -> Result<()> {
        writer.write_octet_string(self)
    }
}

// Implement BerDecode for primitive types
impl BerDecode for bool {
    fn decode(reader: &mut BerReader) -> Result<Self> {
        reader.read_boolean()
    }
}

impl BerDecode for i64 {
    fn decode(reader: &mut BerReader) -> Result<Self> {
        reader.read_integer()
    }
}

impl BerDecode for i32 {
    fn decode(reader: &mut BerReader) -> Result<Self> {
        let value = reader.read_integer()?;
        i32::try_from(value).map_err(|_| BerError::IntegerConversion.into())
    }
}

impl BerDecode for f64 {
    fn decode(reader: &mut BerReader) -> Result<Self> {
        reader.read_real()
    }
}

impl BerDecode for String {
    fn decode(reader: &mut BerReader) -> Result<Self> {
        reader.read_utf8_string()
    }
}

impl BerDecode for Vec<u8> {
    fn decode(reader: &mut BerReader) -> Result<Self> {
        reader.read_octet_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boolean_roundtrip() {
        let encoded = encode(&true).unwrap();
        let decoded: bool = decode(&encoded).unwrap();
        assert!(decoded);

        let encoded = encode(&false).unwrap();
        let decoded: bool = decode(&encoded).unwrap();
        assert!(!decoded);
    }

    #[test]
    fn test_integer_roundtrip() {
        for value in [0i64, 1, -1, 127, 128, -128, -129, 32767, -32768, i64::MAX, i64::MIN] {
            let encoded = encode(&value).unwrap();
            let decoded: i64 = decode(&encoded).unwrap();
            assert_eq!(value, decoded);
        }
    }

    #[test]
    fn test_string_roundtrip() {
        let value = "Hello, Ember+!".to_string();
        let encoded = encode(&value).unwrap();
        let decoded: String = decode(&encoded).unwrap();
        assert_eq!(value, decoded);
    }
}
