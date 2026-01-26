//! BER tag encoding and decoding.

use crate::error::{BerError, Result};

/// Tag class in BER encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TagClass {
    /// Universal class (0b00)
    Universal = 0,
    /// Application class (0b01)
    Application = 1,
    /// Context-specific class (0b10)
    Context = 2,
    /// Private class (0b11)
    Private = 3,
}

impl TagClass {
    /// Create a tag class from the class bits.
    pub fn from_bits(bits: u8) -> Result<Self> {
        match bits & 0b11 {
            0 => Ok(TagClass::Universal),
            1 => Ok(TagClass::Application),
            2 => Ok(TagClass::Context),
            3 => Ok(TagClass::Private),
            _ => Err(BerError::UnsupportedTagClass(bits).into()),
        }
    }

    /// Get the bits for this tag class.
    pub fn to_bits(self) -> u8 {
        self as u8
    }
}

/// Tag type (primitive or constructed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagType {
    /// Primitive encoding
    Primitive,
    /// Constructed encoding (contains other TLV elements)
    Constructed,
}

impl TagType {
    /// Create from the P/C bit.
    pub fn from_bit(bit: bool) -> Self {
        if bit {
            TagType::Constructed
        } else {
            TagType::Primitive
        }
    }

    /// Convert to the P/C bit.
    pub fn to_bit(self) -> bool {
        matches!(self, TagType::Constructed)
    }
}

/// A BER tag identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tag {
    /// The tag class
    pub class: TagClass,
    /// Whether this is a constructed type
    pub tag_type: TagType,
    /// The tag number
    pub number: u32,
}

impl Tag {
    /// Create a new tag.
    pub const fn new(class: TagClass, tag_type: TagType, number: u32) -> Self {
        Tag { class, tag_type, number }
    }

    /// Create a universal primitive tag.
    pub const fn universal_primitive(number: u32) -> Self {
        Tag::new(TagClass::Universal, TagType::Primitive, number)
    }

    /// Create a universal constructed tag.
    pub const fn universal_constructed(number: u32) -> Self {
        Tag::new(TagClass::Universal, TagType::Constructed, number)
    }

    /// Create a context-specific primitive tag.
    pub const fn context_primitive(number: u32) -> Self {
        Tag::new(TagClass::Context, TagType::Primitive, number)
    }

    /// Create a context-specific constructed tag.
    pub const fn context_constructed(number: u32) -> Self {
        Tag::new(TagClass::Context, TagType::Constructed, number)
    }

    /// Create an application primitive tag.
    pub const fn application_primitive(number: u32) -> Self {
        Tag::new(TagClass::Application, TagType::Primitive, number)
    }

    /// Create an application constructed tag.
    pub const fn application_constructed(number: u32) -> Self {
        Tag::new(TagClass::Application, TagType::Constructed, number)
    }

    /// Check if this tag is primitive.
    pub fn is_primitive(&self) -> bool {
        self.tag_type == TagType::Primitive
    }

    /// Check if this tag is constructed.
    pub fn is_constructed(&self) -> bool {
        self.tag_type == TagType::Constructed
    }

    /// Check if this tag is universal class.
    pub fn is_universal(&self) -> bool {
        self.class == TagClass::Universal
    }

    /// Check if this tag is context-specific class.
    pub fn is_context(&self) -> bool {
        self.class == TagClass::Context
    }

    /// Check if this tag is application class.
    pub fn is_application(&self) -> bool {
        self.class == TagClass::Application
    }

    /// Encode this tag to bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut result = Vec::new();
        
        // First byte: class (2 bits) | P/C (1 bit) | number or 0x1F (5 bits)
        let first_byte = (self.class.to_bits() << 6)
            | ((self.tag_type.to_bit() as u8) << 5)
            | if self.number < 31 { self.number as u8 } else { 0x1F };
        
        result.push(first_byte);
        
        // Long form tag number if >= 31
        if self.number >= 31 {
            let mut number = self.number;
            let mut bytes = Vec::new();
            
            // Encode in base-128, most significant byte first
            loop {
                bytes.push((number & 0x7F) as u8);
                number >>= 7;
                if number == 0 {
                    break;
                }
            }
            
            // Reverse and set continuation bits
            for (i, byte) in bytes.iter().rev().enumerate() {
                if i < bytes.len() - 1 {
                    result.push(byte | 0x80); // Set continuation bit
                } else {
                    result.push(*byte); // Last byte, no continuation
                }
            }
        }
        
        result
    }

    /// Decode a tag from bytes.
    pub fn decode(data: &[u8]) -> Result<(Self, usize)> {
        if data.is_empty() {
            return Err(BerError::UnexpectedEof.into());
        }

        let first_byte = data[0];
        let class = TagClass::from_bits(first_byte >> 6)?;
        let tag_type = TagType::from_bit((first_byte & 0x20) != 0);
        let short_number = first_byte & 0x1F;

        if short_number < 31 {
            // Short form
            Ok((Tag::new(class, tag_type, short_number as u32), 1))
        } else {
            // Long form
            let mut number: u32 = 0;
            let mut pos = 1;

            loop {
                if pos >= data.len() {
                    return Err(BerError::UnexpectedEof.into());
                }

                let byte = data[pos];
                number = number
                    .checked_shl(7)
                    .ok_or(BerError::LengthOverflow)?
                    .checked_add((byte & 0x7F) as u32)
                    .ok_or(BerError::LengthOverflow)?;
                pos += 1;

                if (byte & 0x80) == 0 {
                    break;
                }
            }

            Ok((Tag::new(class, tag_type, number), pos))
        }
    }
}

// Common tag constants
impl Tag {
    /// BOOLEAN tag
    pub const BOOLEAN: Tag = Tag::universal_primitive(1);
    /// INTEGER tag
    pub const INTEGER: Tag = Tag::universal_primitive(2);
    /// BIT STRING tag
    pub const BIT_STRING: Tag = Tag::universal_primitive(3);
    /// OCTET STRING tag
    pub const OCTET_STRING: Tag = Tag::universal_primitive(4);
    /// NULL tag
    pub const NULL: Tag = Tag::universal_primitive(5);
    /// OBJECT IDENTIFIER tag
    pub const OID: Tag = Tag::universal_primitive(6);
    /// REAL tag
    pub const REAL: Tag = Tag::universal_primitive(9);
    /// ENUMERATED tag
    pub const ENUMERATED: Tag = Tag::universal_primitive(10);
    /// UTF8String tag
    pub const UTF8_STRING: Tag = Tag::universal_primitive(12);
    /// RELATIVE-OID tag
    pub const RELATIVE_OID: Tag = Tag::universal_primitive(13);
    /// SEQUENCE tag
    pub const SEQUENCE: Tag = Tag::universal_constructed(16);
    /// SET tag
    pub const SET: Tag = Tag::universal_constructed(17);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_tag_roundtrip() {
        for number in 0..31u32 {
            let tag = Tag::universal_primitive(number);
            let encoded = tag.encode();
            let (decoded, len) = Tag::decode(&encoded).unwrap();
            assert_eq!(len, 1);
            assert_eq!(tag, decoded);
        }
    }

    #[test]
    fn test_long_tag_roundtrip() {
        for number in [31u32, 127, 128, 255, 256, 16383, 16384, 0x1FFFFF] {
            let tag = Tag::context_constructed(number);
            let encoded = tag.encode();
            let (decoded, _) = Tag::decode(&encoded).unwrap();
            assert_eq!(tag, decoded);
        }
    }

    #[test]
    fn test_tag_classes() {
        let tag = Tag::new(TagClass::Application, TagType::Constructed, 5);
        assert!(tag.is_application());
        assert!(tag.is_constructed());
        
        let tag = Tag::context_primitive(10);
        assert!(tag.is_context());
        assert!(tag.is_primitive());
    }
}
