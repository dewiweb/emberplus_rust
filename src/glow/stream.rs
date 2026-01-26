//! Stream handling for Ember+ streaming parameters.

use super::element::{EmberValue, StreamFormat};
use super::root::StreamEntry;
use crate::error::Result;

/// Stream manager for handling streaming parameter values.
#[derive(Debug, Default)]
pub struct StreamManager {
    /// Active stream subscriptions
    subscriptions: std::collections::HashMap<i32, StreamSubscription>,
}

/// A stream subscription.
#[derive(Debug, Clone)]
pub struct StreamSubscription {
    /// Stream identifier
    pub stream_id: i32,
    /// Stream format
    pub format: StreamFormat,
    /// Last received value
    pub last_value: Option<EmberValue>,
}

impl StreamManager {
    /// Create a new stream manager.
    pub fn new() -> Self {
        StreamManager::default()
    }

    /// Subscribe to a stream.
    pub fn subscribe(&mut self, stream_id: i32, format: StreamFormat) {
        self.subscriptions.insert(stream_id, StreamSubscription {
            stream_id,
            format,
            last_value: None,
        });
    }

    /// Unsubscribe from a stream.
    pub fn unsubscribe(&mut self, stream_id: i32) {
        self.subscriptions.remove(&stream_id);
    }

    /// Process a stream entry update.
    pub fn process_entry(&mut self, entry: &StreamEntry) {
        if let Some(sub) = self.subscriptions.get_mut(&entry.stream_identifier) {
            sub.last_value = Some(entry.value.clone());
        }
    }

    /// Get the last value for a stream.
    pub fn get_value(&self, stream_id: i32) -> Option<&EmberValue> {
        self.subscriptions.get(&stream_id).and_then(|s| s.last_value.as_ref())
    }

    /// Check if subscribed to a stream.
    pub fn is_subscribed(&self, stream_id: i32) -> bool {
        self.subscriptions.contains_key(&stream_id)
    }

    /// Get all active subscriptions.
    pub fn subscriptions(&self) -> impl Iterator<Item = &StreamSubscription> {
        self.subscriptions.values()
    }
}

/// Decode a stream value based on format.
pub fn decode_stream_value(data: &[u8], format: StreamFormat, offset: usize) -> Option<EmberValue> {
    if offset >= data.len() {
        return None;
    }
    let data = &data[offset..];
    
    match format {
        StreamFormat::UInt8 => {
            data.first().map(|&v| EmberValue::Integer(v as i64))
        }
        StreamFormat::Int8 => {
            data.first().map(|&v| EmberValue::Integer(v as i8 as i64))
        }
        StreamFormat::UInt16BigEndian => {
            if data.len() >= 2 {
                Some(EmberValue::Integer(u16::from_be_bytes([data[0], data[1]]) as i64))
            } else {
                None
            }
        }
        StreamFormat::UInt16LittleEndian => {
            if data.len() >= 2 {
                Some(EmberValue::Integer(u16::from_le_bytes([data[0], data[1]]) as i64))
            } else {
                None
            }
        }
        StreamFormat::Int16BigEndian => {
            if data.len() >= 2 {
                Some(EmberValue::Integer(i16::from_be_bytes([data[0], data[1]]) as i64))
            } else {
                None
            }
        }
        StreamFormat::Int16LittleEndian => {
            if data.len() >= 2 {
                Some(EmberValue::Integer(i16::from_le_bytes([data[0], data[1]]) as i64))
            } else {
                None
            }
        }
        StreamFormat::UInt32BigEndian => {
            if data.len() >= 4 {
                Some(EmberValue::Integer(u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as i64))
            } else {
                None
            }
        }
        StreamFormat::UInt32LittleEndian => {
            if data.len() >= 4 {
                Some(EmberValue::Integer(u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as i64))
            } else {
                None
            }
        }
        StreamFormat::Int32BigEndian => {
            if data.len() >= 4 {
                Some(EmberValue::Integer(i32::from_be_bytes([data[0], data[1], data[2], data[3]]) as i64))
            } else {
                None
            }
        }
        StreamFormat::Int32LittleEndian => {
            if data.len() >= 4 {
                Some(EmberValue::Integer(i32::from_le_bytes([data[0], data[1], data[2], data[3]]) as i64))
            } else {
                None
            }
        }
        StreamFormat::Float32BigEndian => {
            if data.len() >= 4 {
                Some(EmberValue::Real(f32::from_be_bytes([data[0], data[1], data[2], data[3]]) as f64))
            } else {
                None
            }
        }
        StreamFormat::Float32LittleEndian => {
            if data.len() >= 4 {
                Some(EmberValue::Real(f32::from_le_bytes([data[0], data[1], data[2], data[3]]) as f64))
            } else {
                None
            }
        }
        StreamFormat::Float64BigEndian => {
            if data.len() >= 8 {
                let arr: [u8; 8] = data[..8].try_into().ok()?;
                Some(EmberValue::Real(f64::from_be_bytes(arr)))
            } else {
                None
            }
        }
        StreamFormat::Float64LittleEndian => {
            if data.len() >= 8 {
                let arr: [u8; 8] = data[..8].try_into().ok()?;
                Some(EmberValue::Real(f64::from_le_bytes(arr)))
            } else {
                None
            }
        }
        _ => None,
    }
}
