//! BER writer for encoding data to BER format.

use super::{Tag, Length, TagClass, TagType};
use crate::error::Result;

/// A writer for BER-encoded data.
#[derive(Debug, Default)]
pub struct BerWriter {
    buffer: Vec<u8>,
}

impl BerWriter {
    /// Create a new BER writer.
    pub fn new() -> Self {
        BerWriter { buffer: Vec::new() }
    }

    /// Create a new BER writer with initial capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        BerWriter { buffer: Vec::with_capacity(capacity) }
    }

    /// Get the written bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }

    /// Consume the writer and return the bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.buffer
    }

    /// Get the current length.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Write raw bytes.
    pub fn write_raw(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    /// Write a tag.
    pub fn write_tag(&mut self, tag: &Tag) {
        self.buffer.extend(tag.encode());
    }

    /// Write a length.
    pub fn write_length(&mut self, length: &Length) {
        self.buffer.extend(length.encode());
    }

    /// Write a TLV with the given tag and value.
    pub fn write_tlv(&mut self, tag: &Tag, value: &[u8]) {
        self.write_tag(tag);
        self.write_length(&Length::Definite(value.len()));
        self.write_raw(value);
    }

    /// Write a boolean value.
    pub fn write_boolean(&mut self, value: bool) -> Result<()> {
        self.write_tlv(&Tag::BOOLEAN, &[if value { 0xFF } else { 0x00 }]);
        Ok(())
    }

    /// Write an integer value.
    pub fn write_integer(&mut self, value: i64) -> Result<()> {
        let bytes = Self::encode_integer(value);
        self.write_tlv(&Tag::INTEGER, &bytes);
        Ok(())
    }

    /// Write an integer with a context tag.
    pub fn write_context_integer(&mut self, tag_number: u32, value: i64) -> Result<()> {
        let tag = Tag::new(TagClass::Context, TagType::Primitive, tag_number);
        let bytes = Self::encode_integer(value);
        self.write_tlv(&tag, &bytes);
        Ok(())
    }

    /// Encode an integer to bytes.
    fn encode_integer(value: i64) -> Vec<u8> {
        if value == 0 {
            return vec![0];
        }

        let mut bytes = Vec::new();
        let mut remaining = value;
        
        // Handle both positive and negative numbers
        if value > 0 {
            while remaining > 0 {
                bytes.push((remaining & 0xFF) as u8);
                remaining >>= 8;
            }
            // Add leading zero if high bit is set (would look negative)
            if !bytes.is_empty() && (bytes.last().unwrap() & 0x80) != 0 {
                bytes.push(0);
            }
        } else {
            // Negative numbers
            while remaining != -1 || bytes.is_empty() || (bytes.last().unwrap() & 0x80) == 0 {
                bytes.push((remaining & 0xFF) as u8);
                remaining >>= 8;
                if bytes.len() >= 8 {
                    break;
                }
            }
        }
        
        bytes.reverse();
        bytes
    }

    /// Write a real (floating point) value.
    pub fn write_real(&mut self, value: f64) -> Result<()> {
        let bytes = Self::encode_real(value);
        self.write_tlv(&Tag::REAL, &bytes);
        Ok(())
    }

    /// Encode a real to bytes.
    fn encode_real(value: f64) -> Vec<u8> {
        if value == 0.0 {
            if value.is_sign_negative() {
                return vec![0x43]; // Minus zero
            }
            return vec![]; // Plus zero
        }
        
        if value.is_infinite() {
            return if value.is_sign_positive() {
                vec![0x40] // Plus infinity
            } else {
                vec![0x41] // Minus infinity
            };
        }
        
        if value.is_nan() {
            return vec![0x42]; // NaN
        }
        
        // Use IEEE 754 double-precision encoding
        let bits = value.to_bits();
        let sign = (bits >> 63) as u8;
        let exponent = ((bits >> 52) & 0x7FF) as i16 - 1023 - 52;
        let mantissa = if ((bits >> 52) & 0x7FF) == 0 {
            bits & 0xFFFFFFFFFFFFF // Denormalized
        } else {
            (bits & 0xFFFFFFFFFFFFF) | 0x10000000000000 // Add implicit bit
        };
        
        // Remove trailing zeros from mantissa
        let mut m = mantissa;
        let mut exp = exponent;
        while m != 0 && (m & 1) == 0 {
            m >>= 1;
            exp += 1;
        }
        
        // Encode mantissa bytes
        let mut mantissa_bytes = Vec::new();
        let mut temp = m;
        while temp > 0 {
            mantissa_bytes.push((temp & 0xFF) as u8);
            temp >>= 8;
        }
        mantissa_bytes.reverse();
        
        // Encode exponent bytes
        let exp_bytes = Self::encode_integer(exp as i64);
        
        // Build result
        let mut result = Vec::new();
        
        // First byte: binary encoding, base 2, scale 0
        let mut first_byte: u8 = 0x80; // Binary encoding
        if sign != 0 {
            first_byte |= 0x40; // Negative
        }
        
        match exp_bytes.len() {
            1 => first_byte |= 0x00,
            2 => first_byte |= 0x01,
            3 => first_byte |= 0x02,
            _ => first_byte |= 0x03,
        }
        
        result.push(first_byte);
        
        if exp_bytes.len() > 3 {
            result.push(exp_bytes.len() as u8);
        }
        
        result.extend(&exp_bytes);
        result.extend(&mantissa_bytes);
        
        result
    }

    /// Write a UTF-8 string.
    pub fn write_utf8_string(&mut self, value: &str) -> Result<()> {
        self.write_tlv(&Tag::UTF8_STRING, value.as_bytes());
        Ok(())
    }

    /// Write an octet string.
    pub fn write_octet_string(&mut self, value: &[u8]) -> Result<()> {
        self.write_tlv(&Tag::OCTET_STRING, value);
        Ok(())
    }

    /// Write a null value.
    pub fn write_null(&mut self) -> Result<()> {
        self.write_tlv(&Tag::NULL, &[]);
        Ok(())
    }

    /// Write a sequence.
    pub fn write_sequence<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut BerWriter) -> Result<()>,
    {
        let mut inner = BerWriter::new();
        f(&mut inner)?;
        self.write_tlv(&Tag::SEQUENCE, inner.as_bytes());
        Ok(())
    }

    /// Write a set.
    pub fn write_set<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut BerWriter) -> Result<()>,
    {
        let mut inner = BerWriter::new();
        f(&mut inner)?;
        self.write_tlv(&Tag::SET, inner.as_bytes());
        Ok(())
    }

    /// Write a UTF8String.
    pub fn write_utf8string(&mut self, value: &str) {
        self.write_tlv(&Tag::UTF8_STRING, value.as_bytes());
    }

    /// Write a context-tagged constructed value.
    pub fn write_context<F>(&mut self, tag_number: u32, f: F) -> Result<()>
    where
        F: FnOnce(&mut BerWriter) -> Result<()>,
    {
        let tag = Tag::context_constructed(tag_number);
        let mut inner = BerWriter::new();
        f(&mut inner)?;
        self.write_tlv(&tag, inner.as_bytes());
        Ok(())
    }

    /// Write a context-tagged primitive value.
    pub fn write_context_primitive(&mut self, tag_number: u32, value: &[u8]) -> Result<()> {
        let tag = Tag::context_primitive(tag_number);
        self.write_tlv(&tag, value);
        Ok(())
    }

    /// Write an application-tagged constructed value.
    pub fn write_application<F>(&mut self, tag_number: u32, f: F) -> Result<()>
    where
        F: FnOnce(&mut BerWriter) -> Result<()>,
    {
        let tag = Tag::application_constructed(tag_number);
        let mut inner = BerWriter::new();
        f(&mut inner)?;
        self.write_tlv(&tag, inner.as_bytes());
        Ok(())
    }

    /// Write an application-tagged primitive value.
    pub fn write_application_primitive(&mut self, tag_number: u32, value: &[u8]) -> Result<()> {
        let tag = Tag::application_primitive(tag_number);
        self.write_tlv(&tag, value);
        Ok(())
    }

    /// Write a relative OID.
    pub fn write_relative_oid(&mut self, components: &[u32]) -> Result<()> {
        let mut bytes = Vec::new();
        
        for &component in components {
            let mut temp = Vec::new();
            let mut value = component;
            
            temp.push((value & 0x7F) as u8);
            value >>= 7;
            
            while value > 0 {
                temp.push(((value & 0x7F) | 0x80) as u8);
                value >>= 7;
            }
            
            temp.reverse();
            bytes.extend(temp);
        }
        
        self.write_tlv(&Tag::RELATIVE_OID, &bytes);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_encoding() {
        // Test cases: (value, expected_bytes)
        let cases = [
            (0i64, vec![0x00]),
            (1, vec![0x01]),
            (127, vec![0x7F]),
            (128, vec![0x00, 0x80]),
            (256, vec![0x01, 0x00]),
            (-1, vec![0xFF]),
            (-128, vec![0x80]),
            (-129, vec![0xFF, 0x7F]),
        ];
        
        for (value, expected) in cases {
            let encoded = BerWriter::encode_integer(value);
            assert_eq!(encoded, expected, "failed for value {}", value);
        }
    }
}
