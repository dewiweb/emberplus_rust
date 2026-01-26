//! Server event handlers.

use std::sync::Arc;
use crate::error::Result;
use crate::glow::{EmberPath, GlowConnection, InvocationResult, EmberValue};

/// Handler for setValue operations.
pub type SetValueHandler = Arc<dyn Fn(&EmberPath, &EmberValue) -> Result<bool> + Send + Sync>;

/// Handler for function invocations.
pub type InvokeHandler = Arc<dyn Fn(&EmberPath, i32, &[EmberValue]) -> Result<InvocationResult> + Send + Sync>;

/// Handler for matrix operations.
pub type MatrixHandler = Arc<dyn Fn(&EmberPath, &GlowConnection) -> Result<()> + Send + Sync>;

/// Default handler that accepts all setValue operations.
pub fn default_set_value_handler() -> SetValueHandler {
    Arc::new(|_path, _value| Ok(true))
}

/// Default handler that returns success for all invocations.
pub fn default_invoke_handler() -> InvokeHandler {
    Arc::new(|_path, invocation_id, _args| {
        Ok(InvocationResult::success(invocation_id, vec![]))
    })
}

/// Default handler for matrix operations.
pub fn default_matrix_handler() -> MatrixHandler {
    Arc::new(|_path, _connection| Ok(()))
}
