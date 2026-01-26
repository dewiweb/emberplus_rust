# Ember+ Rust Implementation

A complete Rust implementation of [Lawo's Ember+ control protocol](https://github.com/Lawo/ember-plus).

Ember+ is a control protocol used in broadcast and professional audio/video equipment, providing a tree-based data structure for parameters, nodes, functions, and matrices.

## Features

- **Complete Protocol Support**
  - S101 framing protocol (transport layer)
  - ASN.1 BER encoding/decoding
  - Glow DTD (Ember+ schema)
  
- **Client (Consumer)**
  - Connect to Ember+ providers
  - Browse tree structure
  - Read/write parameter values
  - Subscribe to changes
  - Invoke functions
  - Matrix operations

- **Server (Provider)**
  - Host an Ember+ tree
  - Handle client connections
  - Callbacks for value changes
  - Function invocation handlers
  - Matrix operation handlers

- **Async/Await**
  - Built on Tokio for efficient async I/O
  - Non-blocking operations

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
ember-plus = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Quick Start

### Client Example

```rust
use ember_plus::{EmberClient, EmberValue, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to a provider
    let client = EmberClient::connect("192.168.1.100:9000").await?;
    
    // Get the root directory
    let root = client.get_directory().await?;
    
    // Get element by path
    let node = client.get_element_by_path("0.1.2").await?;
    
    // Set a parameter value
    client.set_value("0.1.2", EmberValue::Integer(42)).await?;
    
    // Subscribe to changes
    client.subscribe("0.1").await?;
    
    // Register callback for updates
    client.on_value_change(Box::new(|path, value| {
        println!("Value changed: {:?} = {}", path, value);
    })).await;
    
    // Invoke a function
    let result = client.invoke("0.2.1", vec![
        EmberValue::Integer(1),
        EmberValue::String("test".into()),
    ]).await?;
    
    client.disconnect().await?;
    Ok(())
}
```

### Server Example

```rust
use std::sync::Arc;
use ember_plus::{EmberServer, EmberValue, InvocationResult, Result};
use ember_plus::tree::TreeNode;

#[tokio::main]
async fn main() -> Result<()> {
    let mut server = EmberServer::bind("0.0.0.0:9000").await?;
    
    // Build your tree
    let mut root = TreeNode::new_node(0);
    // ... add children, parameters, etc.
    
    server.init(root).await;
    
    // Handle setValue operations
    server.set_value_handler(Arc::new(|path, value| {
        println!("setValue: {:?} = {}", path, value);
        Ok(true) // Accept the change
    }));
    
    // Handle function invocations
    server.invoke_handler(Arc::new(|path, id, args| {
        Ok(InvocationResult::success(id, vec![]))
    }));
    
    server.run().await?;
    Ok(())
}
```

## CLI Tool

The crate includes a CLI tool for testing:

```bash
# Connect to a provider
cargo run -- client 192.168.1.100:9000

# Start a test server
cargo run -- server 9000
```

## Examples

See the `examples/` directory for more detailed examples:

```bash
# Run the client example
cargo run --example client -- localhost:9000

# Run the server example
cargo run --example server -- 9000
```

## Module Structure

- `ber` - ASN.1 BER encoding/decoding
- `s101` - S101 framing protocol
- `glow` - Glow DTD types and tags
- `tree` - Runtime tree data structures
- `codec` - High-level Glow encoding/decoding
- `client` - Ember+ client (consumer)
- `server` - Ember+ server (provider)

## Protocol Compatibility

This implementation aims to be compatible with:
- Lawo's official Ember+ SDK
- [sofie-emberplus-connection](https://github.com/Sofie-Automation/sofie-emberplus-connection)
- Other compliant Ember+ implementations

Tested with:
- Lawo Ruby
- Lawo R3lay
- Lawo MxGUI

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## References

- [Ember+ Official Repository](https://github.com/Lawo/ember-plus)
- [Ember+ Wiki](https://github.com/Lawo/ember-plus/wiki)
- [Glow DTD Specification](https://github.com/Lawo/ember-plus/tree/master/documentation)
