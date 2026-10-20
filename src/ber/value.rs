//! BER value types.

use super::{Tag, BerReader, BerWriter};
use crate::error::Result;

/// A generic BER value that can hold any type.
#[derive(Debug, Clone, PartialEq)]
pub enum BerValue {
    /// Boolean value
    Boolean(bool),
    /// Integer value
    Integer(i64),
    /// Real (floating point) value
    Real(f64),
    /// UTF-8 string value
    String(String),
    /// Octet string (raw bytes) value
    OctetString(Vec<u8>),
    /// Null value
    Null,
    /// Sequence of values
    Sequence(Vec<BerValue>),
    /// Context-tagged value
    Context {
        /// Context tag number
        tag: u32,
        /// Tagged value
        value: Box<BerValue>,
    },
    /// Application-tagged value
    Application {
        /// Application tag number
        tag: u32,
        /// Tagged value
        value: Box<BerValue>,
    },
    /// Raw TLV (for unknown or unprocessed data)
    Raw {
        /// Raw tag
        tag: Tag,
        /// Raw value bytes
        data: Vec<u8>,
    },
}

impl BerValue {
    /// Try to get this value as a boolean.
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            BerValue::Boolean(v) => Some(*v),
            _ => None,
        }
    }

    /// Try to get this value as an integer.
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            BerValue::Integer(v) => Some(*v),
            _ => None,
        }
    }

    /// Try to get this value as a real.
    pub fn as_real(&self) -> Option<f64> {
        match self {
            BerValue::Real(v) => Some(*v),
            BerValue::Integer(v) => Some(*v as f64),
            _ => None,
        }
    }

    /// Try to get this value as a string.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            BerValue::String(v) => Some(v),
            _ => None,
        }
    }

    /// Try to get this value as an octet string.
    pub fn as_octet_string(&self) -> Option<&[u8]> {
        match self {
            BerValue::OctetString(v) => Some(v),
            _ => None,
        }
    }

    /// Try to get this value as a sequence.
    pub fn as_sequence(&self) -> Option<&[BerValue]> {
        match self {
            BerValue::Sequence(v) => Some(v),
            _ => None,
        }
    }

    /// Check if this value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, BerValue::Null)
    }

    /// Decode a BER value from bytes.
    pub fn decode(data: &[u8]) -> Result<Self> {
        let mut reader = BerReader::new(data);
        Self::decode_from_reader(&mut reader)
    }

    /// Decode a BER value from a reader.
    pub fn decode_from_reader(reader: &mut BerReader<'_>) -> Result<Self> {
        let (tag, value_bytes) = reader.read_tlv()?;
        
        if tag.is_universal() {
            match tag.number {
                1 => {
                    // BOOLEAN
                    Ok(BerValue::Boolean(value_bytes.first().map(|&b| b != 0).unwrap_or(false)))
                }
                2 => {
                    // INTEGER
                    // For raw value bytes, decode directly
                    let value = decode_integer_bytes(value_bytes)?;
                    Ok(BerValue::Integer(value))
                }
                5 => {
                    // NULL
                    Ok(BerValue::Null)
                }
                9 => {
                    // REAL
                    let value = decode_real_bytes(value_bytes)?;
                    Ok(BerValue::Real(value))
                }
                12 => {
                    // UTF8String
                    Ok(BerValue::String(String::from_utf8(value_bytes.to_vec())?))
                }
                4 => {
                    // OCTET STRING
                    Ok(BerValue::OctetString(value_bytes.to_vec()))
                }
                16 => {
                    // SEQUENCE
                    let mut items = Vec::new();
                    while inner.has_more() {
                        items.push(Self::decode_from_reader(&mut inner)?);
                    }
                    Ok(BerValue::Sequence(items))
                }
                _ => {
                    // Unknown universal tag
                    Ok(BerValue::Raw { tag, data: value_bytes.to_vec() })
                }
            }
        } else if tag.is_context() {
            let inner_value = if tag.is_constructed() && !value_bytes.is_empty() {
                Self::decode(value_bytes)?
            } else {
                BerValue::OctetString(value_bytes.to_vec())
            };
            Ok(BerValue::Context {
                tag: tag.number,
                value: Box::new(inner_value),
            })
        } else if tag.is_application() {
            let inner_value = if tag.is_constructed() && !value_bytes.is_empty() {
                Self::decode(value_bytes)?
            } else {
                BerValue::OctetString(value_bytes.to_vec())
            };
            Ok(BerValue::Application {
                tag: tag.number,
                value: Box::new(inner_value),
            })
        } else {
            Ok(BerValue::Raw { tag, data: value_bytes.to_vec() })
        }
    }

    /// Encode this value to bytes.
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut writer = BerWriter::new();
        self.encode_to_writer(&mut writer)?;
        Ok(writer.into_bytes())
    }

    /// Encode this value to a writer.
    pub fn encode_to_writer(&self, writer: &mut BerWriter) -> Result<()> {
        match self {
            BerValue::Boolean(v) => writer.write_boolean(*v),
            BerValue::Integer(v) => writer.write_integer(*v),
            BerValue::Real(v) => writer.write_real(*v),
            BerValue::String(v) => writer.write_utf8_string(v),
            BerValue::OctetString(v) => writer.write_octet_string(v),
            BerValue::Null => writer.write_null(),
            BerValue::Sequence(items) => {
                writer.write_sequence(|w| {
                    for item in items {
                        item.encode_to_writer(w)?;
                    }
                    Ok(())
                })
            }
            BerValue::Context { tag, value } => {
                writer.write_context(*tag, |w| value.encode_to_writer(w))
            }
            BerValue::Application { tag, value } => {
                writer.write_application(*tag, |w| value.encode_to_writer(w))
            }
            BerValue::Raw { tag, data } => {
                writer.write_tlv(tag, data);
                Ok(())
            }
        }
    }
}

/// Decode an integer from raw value bytes.
fn decode_integer_bytes(value: &[u8]) -> Result<i64> {
    use crate::error::BerError;
    
    if value.is_empty() {
        return Err(BerError::invalid_value("INTEGER", "empty value").into());
    }
    
    if value.len() > 8 {
        return Err(BerError::invalid_value(
            "INTEGER",
            format!("value too large: {} bytes", value.len())
        ).into());
    }
    
    let negative = (value[0] & 0x80) != 0;
    let mut result: i64 = if negative { -1 } else { 0 };
    
    for &byte in value {
        result = (result << 8) | (byte as i64);
    }
    
    Ok(result)
}

/// Decode a real from raw value bytes.
fn decode_real_bytes(value: &[u8]) -> Result<f64> {
    use crate::error::BerError;
    
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
        let sign = if (first_byte & 0x40) != 0 { -1.0 } else { 1.0 };
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
    
    Err(BerError::invalid_value("REAL", "unsupported encoding").into())
}

impl From<bool> for BerValue {
    fn from(v: bool) -> Self {
        BerValue::Boolean(v)
    }
}

impl From<i64> for BerValue {
    fn from(v: i64) -> Self {
        BerValue::Integer(v)
    }
}

impl From<i32> for BerValue {
    fn from(v: i32) -> Self {
        BerValue::Integer(v as i64)
    }
}

impl From<f64> for BerValue {
    fn from(v: f64) -> Self {
        BerValue::Real(v)
    }
}

impl From<String> for BerValue {
    fn from(v: String) -> Self {
        BerValue::String(v)
    }
}

impl From<&str> for BerValue {
    fn from(v: &str) -> Self {
        BerValue::String(v.to_string())
    }
}

impl From<Vec<u8>> for BerValue {
    fn from(v: Vec<u8>) -> Self {
        BerValue::OctetString(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_roundtrip() {
        let values = vec![
            BerValue::Boolean(true),
            BerValue::Boolean(false),
            BerValue::Integer(0),
            BerValue::Integer(42),
            BerValue::Integer(-1),
            BerValue::String("Hello".to_string()),
            BerValue::OctetString(vec![1, 2, 3]),
            BerValue::Null,
            BerValue::Sequence(vec![
                BerValue::Integer(1),
                BerValue::String("test".to_string()),
            ]),
        ];

        for value in values {
            let encoded = value.encode().unwrap();
            let decoded = BerValue::decode(&encoded).unwrap();
            assert_eq!(value, decoded);
        }
    }
}
