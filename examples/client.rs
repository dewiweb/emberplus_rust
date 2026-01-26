//! Example Ember+ client.
//!
//! Run with: cargo run --example client -- <host:port>

use ember_plus::{EmberClient, EmberValue, Result};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = std::env::args().collect();
    let address = args.get(1).map(|s| s.as_str()).unwrap_or("localhost:9000");

    println!("Connecting to {}...", address);
    let client = EmberClient::connect(address).await?;
    println!("Connected!");

    // Get root directory
    println!("\n=== Getting Root Directory ===");
    let root = client.get_directory().await?;
    println!("Received {} elements", root.elements.len());

    for element in &root.elements {
        println!("  {:?}", element);
    }

    // Get element by path (node 1 = identity)
    println!("\n=== Getting Element at path 1 ===");
    match client.get_element_by_path("1").await {
        Ok(Some(node)) => {
            let guard = node.read();
            println!("Found: {} ({:?})", 
                guard.identifier().unwrap_or("(no id)"),
                guard.contents()
            );
        }
        Ok(None) => println!("Element not found"),
        Err(e) => println!("Error: {}", e),
    }

    println!("\n=== Client working! ===");

    // Register callback for value changes
    client.on_value_change(Box::new(|path, value| {
        println!("Value changed at {:?}: {}", path, value);
    })).await;

    // Keep running to receive updates
    println!("\nListening for updates... (Ctrl+C to exit)");
    tokio::signal::ctrl_c().await.ok();

    client.disconnect().await?;
    println!("Disconnected.");

    Ok(())
}
