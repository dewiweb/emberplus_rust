//! Ember+ tree data structures.
//!
//! This module provides the runtime tree representation for Ember+ providers
//! and consumers, with support for efficient lookup and modification.

mod node;
mod walker;

pub use node::{EmberTree, TreeNode, TreeNodeRef, NodeContents};
pub use walker::TreeWalker;
