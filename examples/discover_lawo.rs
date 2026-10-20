//! Discover Lawo MC²36 Ember+ tree structure.
//!
//! Run with: cargo run --example discover_lawo

use ember_plus::{EmberClient, Result};
use tracing_subscriber::EnvFilter;

const LAWO_ADDRESS: &str = "192.168.100.199:9000";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("ember_plus=debug".parse().unwrap()))
        .init();

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     LAWO MC²36 - Ember+ Tree Discovery                     ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    println!("Connecting to {}...", LAWO_ADDRESS);

    let client = match EmberClient::connect(LAWO_ADDRESS).await {
        Ok(c) => {
            println!("✓ Connected successfully!");
            c
        }
        Err(e) => {
            println!("✗ Connection failed: {}", e);
            return Err(e);
        }
    };

    println!();
    println!("════════════════════════════════════════════════════════════");
    println!("  STEP 1: Getting Root Directory");
    println!("════════════════════════════════════════════════════════════");

    let root = client.get_directory().await?;
    println!("Root contains {} elements", root.elements.len());
    
    for element in &root.elements {
        print_element(element, 0);
    }

    println!();
    println!("════════════════════════════════════════════════════════════");
    println!("  STEP 2: Exploring Each Root Node");
    println!("════════════════════════════════════════════════════════════");

    // Explore each root element
    for element in &root.elements {
        if let Some(number) = element.number() {
            let path = format!("{}", number);
            println!();
            println!("▼ Exploring path: {}", path);
            println!("─────────────────────────────────────────────────────────");
            
            explore_path(&client, &path, 1, 3).await;
        }
    }

    println!();
    println!("════════════════════════════════════════════════════════════");
    println!("  STEP 3: Deep Exploration (looking for faders/meters)");
    println!("════════════════════════════════════════════════════════════");

    // Try common Lawo paths for faders and meters
    let paths_to_try = vec![
        "0", "1", "2", "3", "4", "5",
        "0.0", "0.1", "0.2",
        "1.0", "1.1", "1.2",
        "2.0", "2.1", "2.2",
    ];

    for path in paths_to_try {
        match client.get_element_by_path(path).await {
            Ok(Some(node)) => {
                let guard = node.read();
                println!();
                println!("Path [{}]:", path);
                println!("  Identifier: {}", guard.identifier().unwrap_or("(none)"));
                println!("  Description: {}", guard.description().unwrap_or("(none)"));
                let contents = guard.contents();
                println!("  Contents: {:?}", contents);
            }
            Ok(None) => {}
            Err(_) => {}
        }
    }

    println!();
    println!("════════════════════════════════════════════════════════════");
    println!("  Discovery Complete");
    println!("════════════════════════════════════════════════════════════");

    client.disconnect().await?;
    println!("Disconnected.");

    Ok(())
}

async fn explore_path(client: &EmberClient, base_path: &str, depth: usize, max_depth: usize) {
    if depth > max_depth {
        return;
    }

    let indent = "  ".repeat(depth);

    match client.get_element_by_path(base_path).await {
        Ok(Some(node)) => {
            let child_numbers: Vec<i32>;
            {
                let guard = node.read();
                let id = guard.identifier().unwrap_or("(no id)");
                let desc = guard.description().unwrap_or("");
                
                println!("{}├─ [{}] {} {}", indent, base_path, id, 
                    if desc.is_empty() { String::new() } else { format!("- {}", desc) });
                
                let contents = guard.contents();
                println!("{}│  └─ {:?}", indent, contents);

                // Get children numbers
                child_numbers = guard.children().map(|c| c.read().number()).collect();
            }

            for child_num in child_numbers {
                let child_path = format!("{}.{}", base_path, child_num);
                Box::pin(explore_path(client, &child_path, depth + 1, max_depth)).await;
            }
        }
        Ok(None) => {}
        Err(e) => {
            println!("{}├─ [{}] Error: {}", indent, base_path, e);
        }
    }
}

fn print_element(element: &ember_plus::GlowElement, depth: usize) {
    let indent = "  ".repeat(depth);
    
    match element {
        ember_plus::GlowElement::Node(node) => {
            println!("{}[Node #{}] id={:?} desc={:?}", 
                indent, 
                node.number,
                node.identifier,
                node.description
            );
            for child in &node.children {
                print_element(child, depth + 1);
            }
        }
        ember_plus::GlowElement::Parameter(param) => {
            println!("{}[Param #{}] id={:?} value={:?}", 
                indent,
                param.number,
                param.identifier,
                param.value
            );
        }
        ember_plus::GlowElement::QualifiedNode(path, node) => {
            println!("{}[QNode {:?}] id={:?} desc={:?}", 
                indent,
                path,
                node.identifier,
                node.description
            );
            for child in &node.children {
                print_element(child, depth + 1);
            }
        }
        ember_plus::GlowElement::QualifiedParameter(path, param) => {
            println!("{}[QParam {:?}] id={:?} value={:?}", 
                indent,
                path,
                param.identifier,
                param.value
            );
        }
        ember_plus::GlowElement::Matrix(matrix) => {
            println!("{}[Matrix #{}] id={:?} type={:?}", 
                indent,
                matrix.number,
                matrix.identifier,
                matrix.matrix_type
            );
        }
        ember_plus::GlowElement::QualifiedMatrix(path, matrix) => {
            println!("{}[QMatrix {:?}] id={:?}", 
                indent,
                path,
                matrix.identifier
            );
        }
        ember_plus::GlowElement::Function(func) => {
            println!("{}[Func #{}] id={:?}", 
                indent,
                func.number,
                func.identifier
            );
        }
        ember_plus::GlowElement::QualifiedFunction(path, func) => {
            println!("{}[QFunc {:?}] id={:?}", 
                indent,
                path,
                func.identifier
            );
        }
        ember_plus::GlowElement::Template(tmpl) => {
            println!("{}[Template #{}]", indent, tmpl.number);
        }
        ember_plus::GlowElement::Command(cmd) => {
            println!("{}[Command] number={:?}", indent, cmd.number());
        }
    }
}
