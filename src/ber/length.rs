//! BER length encoding and decoding.

use crate::error::{BerError, Result};

/// BER length encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Length {
    /// Definite length
    Definite(usize),
    /// Indefinite length (terminated by end-of-contents)
    Indefinite,
}

impl Length {
    /// Create a definite length.
    pub const fn definite(len: usize) -> Self {
        Length::Definite(len)
    }

    /// Create an indefinite length.
    pub const fn indefinite() -> Self {
        Length::Indefinite
    }

    /// Check if this is a definite length.
    pub fn is_definite(&self) -> bool {
        matches!(self, Length::Definite(_))
    }

    /// Check if this is an indefinite length.
    pub fn is_indefinite(&self) -> bool {
        matches!(self, Length::Indefinite)
    }

    /// Get the definite length value, if any.
    pub fn value(&self) -> Option<usize> {
        match self {
            Length::Definite(len) => Some(*len),
            Length::Indefinite => None,
        }
    }

    /// Encode this length to bytes.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Length::Indefinite => vec![0x80],
            Length::Definite(len) => {
                if *len < 128 {
                    // Short form
                    vec![*len as u8]
                } else {
                    // Long form
                    let mut bytes = Vec::new();
                    let mut remaining = *len;
                    
                    while remaining > 0 {
                        bytes.push((remaining & 0xFF) as u8);
                        remaining >>= 8;
                    }
                    
                    bytes.reverse();
                    
                    let mut result = vec![0x80 | (bytes.len() as u8)];
                    result.extend(bytes);
                    result
                }
            }
        }
    }

    /// Decode a length from bytes.
    pub fn decode(data: &[u8]) -> Result<(Self, usize)> {
        if data.is_empty() {
            return Err(BerError::UnexpectedEof.into());
        }

        let first_byte = data[0];

        if first_byte == 0x80 {
            // Indefinite form
            Ok((Length::Indefinite, 1))
        } else if (first_byte & 0x80) == 0 {
            // Short form
            Ok((Length::Definite(first_byte as usize), 1))
        } else {
            // Long form
            let num_bytes = (first_byte & 0x7F) as usize;
            
            if num_bytes == 0 {
                return Err(BerError::InvalidLength.into());
            }
            
            if num_bytes > 8 {
                return Err(BerError::LengthOverflow.into());
            }
            
            if data.len() < 1 + num_bytes {
                return Err(BerError::UnexpectedEof.into());
            }

            let mut length: usize = 0;
            for i in 0..num_bytes {
                length = length
                    .checked_shl(8)
                    .ok_or(BerError::LengthOverflow)?
                    .checked_add(data[1 + i] as usize)
                    .ok_or(BerError::LengthOverflow)?;
            }

            Ok((Length::Definite(length), 1 + num_bytes))
        }
    }

    /// Calculate the encoded size of a length value.
    pub fn encoded_size(len: usize) -> usize {
        if len < 128 {
            1
        } else {
            let mut size = 1; // Initial byte
            let mut remaining = len;
            while remaining > 0 {
                size += 1;
                remaining >>= 8;
            }
            size
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_length_roundtrip() {
        for len in 0..128usize {
            let length = Length::Definite(len);
            let encoded = length.encode();
            assert_eq!(encoded.len(), 1);
            let (decoded, size) = Length::decode(&encoded).unwrap();
            assert_eq!(size, 1);
            assert_eq!(length, decoded);
        }
    }

    #[test]
    fn test_long_length_roundtrip() {
        for len in [128usize, 255, 256, 65535, 65536, 0xFFFFFF, 0x1000000] {
            let length = Length::Definite(len);
            let encoded = length.encode();
            let (decoded, size) = Length::decode(&encoded).unwrap();
            assert_eq!(size, encoded.len());
            assert_eq!(length, decoded);
        }
    }

    #[test]
    fn test_indefinite_length() {
        let length = Length::Indefinite;
        let encoded = length.encode();
        assert_eq!(encoded, vec![0x80]);
        let (decoded, size) = Length::decode(&encoded).unwrap();
        assert_eq!(size, 1);
        assert_eq!(length, decoded);
    }
}
