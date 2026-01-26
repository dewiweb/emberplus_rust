//! Test matrix reading - use get_element_by_path for progressive navigation.

use ember_plus::{EmberClient, Result};
use ember_plus::glow::GlowElement;

fn element_summary(el: &GlowElement) -> String {
    match el {
        GlowElement::Node(n) => format!("Node({}, id={:?}, children={})", n.number, n.identifier, n.children.len()),
        GlowElement::QualifiedNode(p, n) => format!("QNode({:?}, id={:?}, children={})", p, n.identifier, n.children.len()),
        GlowElement::Matrix(m) => format!("Matrix({}, id={:?})", m.number, m.identifier),
        GlowElement::QualifiedMatrix(p, m) => format!("QMatrix({:?}, id={:?})", p, m.identifier),
        GlowElement::Parameter(p) => format!("Param({}, id={:?})", p.number, p.identifier),
        GlowElement::Command(c) => format!("Command({:?})", c),
        _ => "Other".to_string(),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let address = args.get(1).map(|s| s.as_str()).unwrap_or("192.168.0.39:9000");

    println!("Connecting to {}...", address);
    let client = EmberClient::connect(address).await?;
    println!("Connected!\n");

    // Step 1: Get root
    println!("=== Step 1: Getting Root ===");
    let root = client.get_directory().await?;
    println!("Root has {} elements", root.elements.len());
    for el in &root.elements {
        if let GlowElement::Node(n) = el {
            println!("  {}: {}", n.number, n.identifier.as_deref().unwrap_or("?"));
        }
    }

    // Step 2: Send raw GetDirectory for path [5] and inspect response
    println!("\n=== Step 2: Get facades (5) - with debug ===");
    use ember_plus::glow::{GlowRoot, CommandBuilder};
    let elements = CommandBuilder::get_directory_at_path(&vec![5]);
    let req = GlowRoot::with_elements(elements);
    match client.send_request(req).await {
        Ok(resp) => {
            println!("Response has {} elements", resp.elements.len());
            for (i, el) in resp.elements.iter().enumerate() {
                println!("  [{}] {:?}", i, element_summary(el));
            }
        }
        Err(e) => println!("Error: {}", e),
    }

    // Step 3: Navigate to vsm
    println!("\n=== Step 3: Get vsm (5.1) ===");
    match client.get_element_by_path("5.1").await {
        Ok(Some(node)) => {
            let guard = node.read();
            println!("Found: {}", guard.identifier().unwrap_or("?"));
        }
        Ok(None) => println!("Not found"),
        Err(e) => println!("Error: {}", e),
    }

    // Step 4: Navigate to routing
    println!("\n=== Step 4: Get routing (5.1.1) ===");
    match client.get_element_by_path("5.1.1").await {
        Ok(Some(node)) => {
            let guard = node.read();
            println!("Found: {}", guard.identifier().unwrap_or("?"));
        }
        Ok(None) => println!("Not found"),
        Err(e) => println!("Error: {}", e),
    }

    // Step 5: Navigate to matrix
    println!("\n=== Step 5: Get signalMatrix (5.1.1.1) ===");
    match client.get_element_by_path("5.1.1.1").await {
        Ok(Some(node)) => {
            let guard = node.read();
            println!("Found: {}", guard.identifier().unwrap_or("?"));
            println!("Type: {:?}", guard.contents());
        }
        Ok(None) => println!("Not found"),
        Err(e) => println!("Error: {}", e),
    }

    println!("\nDone!");
    Ok(())
}
