//! Test function invocation.

use ember_plus::{EmberClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let address = args.get(1).map(|s| s.as_str()).unwrap_or("192.168.0.39:9000");

    println!("Connecting to {}...", address);
    let client = EmberClient::connect(address).await?;
    println!("Connected!\n");

    // Get root directory first
    println!("=== Getting Root Directory ===");
    let root = client.get_directory().await?;
    println!("Found {} root elements\n", root.elements.len());

    // Try to invoke getCurrentSnapshot function at path 6.2
    // (functions node is 6, getCurrentSnapshot is child 2)
    println!("=== Invoking getCurrentSnapshot (path 6.2) ===");
    match client.invoke("6.2", vec![]).await {
        Ok(result) => {
            println!("Invocation success!");
            println!("  ID: {}", result.invocation_id);
            println!("  Success: {}", result.success);
            println!("  Result: {:?}", result.result);
        }
        Err(e) => {
            println!("Invocation failed: {}", e);
        }
    }

    // Try getSnapshotList at path 6.1
    println!("\n=== Invoking getSnapshotList (path 6.1) ===");
    match client.invoke("6.1", vec![]).await {
        Ok(result) => {
            println!("Invocation success!");
            println!("  ID: {}", result.invocation_id);
            println!("  Success: {}", result.success);
            println!("  Result: {:?}", result.result);
        }
        Err(e) => {
            println!("Invocation failed: {}", e);
        }
    }

    println!("\nDone!");
    Ok(())
}
