//! Glow DTD (Document Type Definition) types for Ember+.
//!
//! The Glow DTD defines the schema for Ember+ data structures,
//! including nodes, parameters, functions, matrices, and their
//! associated properties.

mod tags;
mod element;
mod root;
mod command;
mod stream;

pub use tags::*;
pub use element::*;
pub use root::*;
pub use command::*;
pub use stream::*;

use crate::error::Result;
use crate::ber::{BerReader, BerWriter, BerEncode, BerDecode};

/// Glow DTD version.
pub const GLOW_DTD_VERSION: (u8, u8) = (2, 31);

/// Minimum schema version supported.
pub const GLOW_MIN_VERSION: (u8, u8) = (2, 0);

/// Schema identifier for Glow.
pub const GLOW_SCHEMA_IDENTIFIER: &str = "EmBER+";
