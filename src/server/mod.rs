//! Ember+ server (provider) implementation.
//!
//! The server hosts an Ember+ tree that clients can:
//! - Browse and discover
//! - Read parameter values
//! - Set parameter values (with callbacks)
//! - Subscribe to changes
//! - Function invocation handlers
//! - Matrix operation handlers

mod listener;
mod handler;

pub use listener::EmberServer;
pub use handler::{SetValueHandler, InvokeHandler, MatrixHandler};
