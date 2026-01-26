//! Ember+ client (consumer) implementation.
//!
//! The client connects to an Ember+ provider and can:
//! - Browse the tree structure
//! - Read and set parameter values
//! - Subscribe to changes
//! - Invoke functions
//! - Monitor matrix connections

mod connection;

pub use connection::EmberClient;
