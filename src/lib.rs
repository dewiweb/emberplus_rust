//! # Ember+ Protocol Implementation for Rust
//!
//! A complete implementation of Lawo's Ember+ control protocol.
//!
//! Ember+ is a control protocol used in broadcast and professional audio
//! equipment, providing a tree-based data structure for parameters, nodes,
//! functions, and matrices.
//!
//! ## Features
//!
//! - **S101 Framing**: Transport layer protocol for message framing
//! - **BER Codec**: ASN.1 Basic Encoding Rules implementation
//! - **Glow DTD**: Complete Ember+ schema type definitions
//! - **Client (Consumer)**: Connect to Ember+ providers
//! - **Server (Provider)**: Host an Ember+ tree for consumers
//! - **Async/Await**: Built on Tokio for async networking
//!
//! ## Quick Start
//!
//! ### Client Example
//!
//! ```rust,no_run
//! use ember_plus::{EmberClient, Result};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let mut client = EmberClient::connect("192.168.1.100:9000").await?;
//!     
//!     // Get the root directory
//!     let tree = client.get_directory().await?;
//!     
//!     // Get element by path
//!     let node = client.get_element_by_path("0.1.2").await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Server Example
//!
//! ```rust,no_run
//! use ember_plus::{EmberServer, EmberValue, Result};
//! use ember_plus::tree::TreeNode;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let mut server = EmberServer::bind("0.0.0.0:9000").await?;
//!     
//!     // Build your tree
//!     let root = TreeNode::new_node(0);
//!     server.init(root).await;
//!     
//!     // Run the server
//!     server.run().await?;
//!     
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod error;
pub mod ber;
pub mod s101;
pub mod glow;
pub mod types;
pub mod tree;
pub mod client;
pub mod server;
pub mod codec;

pub use error::{Error, Result};
pub use types::*;
pub use tree::*;
pub use client::{EmberClient, ChildInfo, ElementType, WorkerPool, WorkerPoolConfig, WorkerTask, TaskResult};
pub use server::EmberServer;
