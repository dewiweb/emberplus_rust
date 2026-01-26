//! BER reader for decoding BER-encoded data.

use super::{Tag, Length};
use crate::error::{BerError, Result};

/// A reader for BER-encoded data.
#[derive(Debug)]
pub struct BerReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BerReader<'a> {
    /// Create a new BER reader.
    pub fn new(data: &'a [u8]) -> Self {
        BerReader { data, pos: 0 }
    }

    /// Get the current position.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Get the remaining bytes.
    pub fn remaining(&self) -> &[u8] {
        &self.data[self.pos..]
    }

    /// Check if there is more data to read.
    pub fn has_more(&self) -> bool {
        self.pos < self.data.len()
    }

    /// Peek at the next tag without advancing.
    pub fn peek_tag(&self) -> Result<Tag> {
        let (tag, _) = Tag::decode(self.remaining())?;
        Ok(tag)
    }

    /// Read a tag.
    pub fn read_tag(&mut self) -> Result<Tag> {
        let (tag, len) = Tag::decode(self.remaining())?;
        self.pos += len;
        Ok(tag)
    }

    /// Read a length.
    pub fn read_length(&mut self) -> Result<Length> {
        let (length, len) = Length::decode(self.remaining())?;
        self.pos += len;
        Ok(length)
    }

    /// Read raw bytes.
    pub fn read_bytes(&mut self, count: usize) -> Result<&'a [u8]> {
        if self.pos + count > self.data.len() {
            return Err(BerError::UnexpectedEof.into());
        }
        let bytes = &self.data[self.pos..self.pos + count];
        self.pos += count;
        Ok(bytes)
    }

    /// Read a TLV (Tag-Length-Value) and return the value bytes.
    pub fn read_tlv(&mut self) -> Result<(Tag, &'a [u8])> {
        let tag = self.read_tag()?;
        let length = self.read_length()?;
        
        let value = match length {
            Length::Definite(len) => self.read_bytes(len)?,
            Length::Indefinite => {
                // Find end-of-contents marker (00 00)
                let start = self.pos;
                loop {
                    if self.pos + 1 >= self.data.len() {
                        return Err(BerError::UnexpectedEof.into());
                    }
                    if self.data[self.pos] == 0x00 && self.data[self.pos + 1] == 0x00 {
                        let value = &self.data[start..self.pos];
                        self.pos += 2; // Skip end-of-contents
                        break value;
                    }
                    self.pos += 1;
                }
            }
        };
        
        Ok((tag, value))
    }

    /// Read a boolean value.
    pub fn read_boolean(&mut self) -> Result<bool> {
        let (tag, value) = self.read_tlv()?;
        
        if tag != Tag::BOOLEAN {
            return Err(BerError::InvalidTag(format!(
                "expected BOOLEAN tag, got {:?}", tag
            )).into());
        }
        
        if value.len() != 1 {
            return Err(BerError::invalid_value(
                "BOOLEAN",
                format!("expected 1 byte, got {}", value.len())
            ).into());
        }
        
        Ok(value[0] != 0)
    }

    /// Read an integer value.
    pub fn read_integer(&mut self) -> Result<i64> {
        let (tag, value) = self.read_tlv()?;
        
        if tag != Tag::INTEGER {
            return Err(BerError::InvalidTag(format!(
                "expected INTEGER tag, got {:?}", tag
            )).into());
        }
        
        Self::decode_integer(value)
    }

    /// Read an integer with a context tag.
    pub fn read_context_integer(&mut self, expected_tag: u32) -> Result<i64> {
        let (tag, value) = self.read_tlv()?;
        
        if !tag.is_context() || tag.number != expected_tag {
            return Err(BerError::InvalidTag(format!(
                "expected context tag [{}], got {:?}", expected_tag, tag
            )).into());
        }
        
        // The value might be a nested INTEGER TLV or raw bytes
        if !value.is_empty() && value[0] == Tag::INTEGER.encode()[0] {
            let mut inner = BerReader::new(value);
            inner.read_integer()
        } else {
            Self::decode_integer(value)
        }
    }

    /// Decode an integer from raw bytes.
    fn decode_integer(value: &[u8]) -> Result<i64> {
        if value.is_empty() {
            return Err(BerError::invalid_value(
                "INTEGER",
                "empty value"
            ).into());
        }
        
        if value.len() > 8 {
            return Err(BerError::invalid_value(
                "INTEGER",
                format!("value too large: {} bytes", value.len())
            ).into());
        }
        
        // Sign-extend if negative
        let negative = (value[0] & 0x80) != 0;
        let mut result: i64 = if negative { -1 } else { 0 };
        
        for &byte in value {
            result = (result << 8) | (byte as i64);
        }
        
        Ok(result)
    }

    /// Read a real (floating point) value.
    pub fn read_real(&mut self) -> Result<f64> {
        let (tag, value) = self.read_tlv()?;
        
        if tag != Tag::REAL {
            return Err(BerError::InvalidTag(format!(
                "expected REAL tag, got {:?}", tag
            )).into());
        }
        
        Self::decode_real(value)
    }

    /// Decode a real from raw bytes.
    fn decode_real(value: &[u8]) -> Result<f64> {
        if value.is_empty() {
            return Ok(0.0);
        }
        
        let first_byte = value[0];
        
        // Special values
        if first_byte == 0x40 {
            return Ok(f64::INFINITY);
        }
        if first_byte == 0x41 {
            return Ok(f64::NEG_INFINITY);
        }
        if first_byte == 0x42 {
            return Ok(f64::NAN);
        }
        if first_byte == 0x43 {
            return Ok(-0.0);
        }
        
        // Binary encoding
        if (first_byte & 0x80) != 0 {
            let sign: f64 = if (first_byte & 0x40) != 0 { -1.0 } else { 1.0 };
            let base: f64 = match (first_byte >> 4) & 0x03 {
                0 => 2.0,
                1 => 8.0,
                2 => 16.0,
                _ => return Err(BerError::invalid_value("REAL", "invalid base").into()),
            };
            let scale_factor = (first_byte >> 2) & 0x03;
            let exponent_length = match first_byte & 0x03 {
                0 => 1,
                1 => 2,
                2 => 3,
                3 => value.get(1).copied().unwrap_or(0) as usize,
                _ => unreachable!(),
            };
            
            let exp_start = if (first_byte & 0x03) == 3 { 2 } else { 1 };
            if value.len() < exp_start + exponent_length {
                return Err(BerError::invalid_value("REAL", "truncated exponent").into());
            }
            
            let exp_bytes = &value[exp_start..exp_start + exponent_length];
            let mut exponent: i64 = if !exp_bytes.is_empty() && (exp_bytes[0] & 0x80) != 0 {
                -1
            } else {
                0
            };
            for &b in exp_bytes {
                exponent = (exponent << 8) | (b as i64);
            }
            
            let mantissa_start = exp_start + exponent_length;
            let mantissa_bytes = &value[mantissa_start..];
            let mut mantissa: u64 = 0;
            for &b in mantissa_bytes {
                mantissa = (mantissa << 8) | (b as u64);
            }
            
            let scaled_mantissa = (mantissa as f64) * (1 << scale_factor) as f64;
            let result = sign * scaled_mantissa * base.powi(exponent as i32);
            
            return Ok(result);
        }
        
        // Decimal encoding (ISO 6093)
        if (first_byte & 0xC0) == 0 {
            let s = std::str::from_utf8(&value[1..])
                .map_err(|_| BerError::invalid_value("REAL", "invalid decimal string"))?;
            return s.trim().parse::<f64>()
                .map_err(|_| BerError::invalid_value("REAL", "invalid decimal value").into());
        }
        
        Err(BerError::invalid_value("REAL", "unsupported encoding").into())
    }

    /// Read a UTF-8 string.
    pub fn read_utf8_string(&mut self) -> Result<String> {
        let (tag, value) = self.read_tlv()?;
        
        if tag != Tag::UTF8_STRING {
            return Err(BerError::InvalidTag(format!(
                "expected UTF8String tag, got {:?}", tag
            )).into());
        }
        
        Ok(String::from_utf8(value.to_vec())?)
    }

    /// Read an octet string.
    pub fn read_octet_string(&mut self) -> Result<Vec<u8>> {
        let (tag, value) = self.read_tlv()?;
        
        if tag != Tag::OCTET_STRING {
            return Err(BerError::InvalidTag(format!(
                "expected OCTET STRING tag, got {:?}", tag
            )).into());
        }
        
        Ok(value.to_vec())
    }

    /// Read a sequence and return a reader for its contents.
    pub fn read_sequence(&mut self) -> Result<BerReader<'a>> {
        let (tag, value) = self.read_tlv()?;
        
        if tag != Tag::SEQUENCE {
            return Err(BerError::InvalidTag(format!(
                "expected SEQUENCE tag, got {:?}", tag
            )).into());
        }
        
        Ok(BerReader::new(value))
    }

    /// Read a context-tagged value and return a reader for its contents.
    pub fn read_context(&mut self, expected_tag: u32) -> Result<BerReader<'a>> {
        let (tag, value) = self.read_tlv()?;
        
        if !tag.is_context() || tag.number != expected_tag {
            return Err(BerError::InvalidTag(format!(
                "expected context tag [{}], got {:?}", expected_tag, tag
            )).into());
        }
        
        Ok(BerReader::new(value))
    }

    /// Read an application-tagged value and return a reader for its contents.
    pub fn read_application(&mut self, expected_tag: u32) -> Result<BerReader<'a>> {
        let (tag, value) = self.read_tlv()?;
        
        if !tag.is_application() || tag.number != expected_tag {
            return Err(BerError::InvalidTag(format!(
                "expected application tag [{}], got {:?}", expected_tag, tag
            )).into());
        }
        
        Ok(BerReader::new(value))
    }

    /// Skip the current TLV.
    pub fn skip(&mut self) -> Result<()> {
        let _ = self.read_tlv()?;
        Ok(())
    }

    /// Read a relative OID.
    pub fn read_relative_oid(&mut self) -> Result<Vec<u32>> {
        let (tag, value) = self.read_tlv()?;
        
        if tag != Tag::RELATIVE_OID {
            return Err(BerError::InvalidTag(format!(
                "expected RELATIVE-OID tag, got {:?}", tag
            )).into());
        }
        
        let mut components = Vec::new();
        let mut current: u32 = 0;
        
        for &byte in value {
            current = current
                .checked_shl(7)
                .ok_or(BerError::LengthOverflow)?
                .checked_add((byte & 0x7F) as u32)
                .ok_or(BerError::LengthOverflow)?;
            
            if (byte & 0x80) == 0 {
                components.push(current);
                current = 0;
            }
        }
        
        Ok(components)
    }
}
