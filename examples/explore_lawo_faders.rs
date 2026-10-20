//! Explore Lawo MC²36 Fader controls in detail.
//!
//! Run with: cargo run --example explore_lawo_faders

use ember_plus::{EmberClient, Result};
use tracing_subscriber::EnvFilter;

const LAWO_ADDRESS: &str = "192.168.100.199:9000";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("ember_plus=info".parse().unwrap()))
        .init();

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     LAWO MC²36 - Fader Discovery (Deep Dive)               ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    println!("Connecting to {}...", LAWO_ADDRESS);
    let client = EmberClient::connect(LAWO_ADDRESS).await?;
    println!("✓ Connected!\n");

    // Explore Inputs (faders) in detail - path 28548.28549
    println!("════════════════════════════════════════════════════════════");
    println!("  EXPLORING: Inputs (Faders) - path: 28548.28549");
    println!("════════════════════════════════════════════════════════════\n");

    explore_node_deep(&client, "28548.28549", 0, 5).await;

    // Also explore Masters
    println!("\n════════════════════════════════════════════════════════════");
    println!("  EXPLORING: Masters - path: 28548.285430");
    println!("════════════════════════════════════════════════════════════\n");

    explore_node_deep(&client, "28548.285430", 0, 5).await;

    // Also explore one specific input channel if we found any
    println!("\n════════════════════════════════════════════════════════════");
    println!("  EXPLORING: First Input Children");
    println!("════════════════════════════════════════════════════════════\n");

    // Get children of Inputs node
    if let Ok(Some(node)) = client.get_element_by_path("28548.28549").await {
        let children: Vec<i32> = {
            let guard = node.read();
            guard.children().map(|c| c.read().number()).collect()
        };
        
        println!("Found {} input channels", children.len());
        
        // Explore first 3 channels in detail
        for (i, &child_num) in children.iter().take(3).enumerate() {
            println!("\n--- Input Channel {} (path: 28548.28549.{}) ---", i + 1, child_num);
            let path = format!("28548.28549.{}", child_num);
            explore_node_deep(&client, &path, 0, 6).await;
        }
    }

    println!("\n════════════════════════════════════════════════════════════");
    println!("  Discovery Complete");
    println!("════════════════════════════════════════════════════════════");

    client.disconnect().await?;
    Ok(())
}

async fn explore_node_deep(client: &EmberClient, path: &str, depth: usize, max_depth: usize) {
    if depth > max_depth {
        println!("{}... (max depth)", "  ".repeat(depth));
        return;
    }

    let indent = "  ".repeat(depth);

    match client.get_element_by_path(path).await {
        Ok(Some(node)) => {
            let child_numbers: Vec<i32>;
            {
                let guard = node.read();
                let id = guard.identifier().unwrap_or("?");
                let desc = guard.description().unwrap_or("");
                let contents = guard.contents();

                match contents {
                    ember_plus::tree::NodeContents::Node { is_online, .. } => {
                        println!("{}📁 [{}] \"{}\" {} online={:?}", 
                            indent, path, id, 
                            if desc.is_empty() { String::new() } else { format!("({})", desc) },
                            is_online);
                    }
                    ember_plus::tree::NodeContents::Parameter { value, minimum, maximum, access, stream_identifier, format, factor, .. } => {
                        println!("{}🎚️  [{}] \"{}\" {}", indent, path, id,
                            if desc.is_empty() { String::new() } else { format!("({})", desc) });
                        if value.is_some() || minimum.is_some() || maximum.is_some() {
                            println!("{}    ↳ value={:?} min={:?} max={:?}", indent, value, minimum, maximum);
                        }
                        if access.is_some() || stream_identifier.is_some() {
                            println!("{}    ↳ access={:?} stream={:?} format={:?} factor={:?}", 
                                indent, access, stream_identifier, format, factor);
                        }
                    }
                    ember_plus::tree::NodeContents::Function { arguments, result, .. } => {
                        println!("{}⚡ [{}] \"{}\" {} (args={}, results={})", 
                            indent, path, id,
                            if desc.is_empty() { String::new() } else { format!("({})", desc) },
                            arguments.len(), result.len());
                    }
                    ember_plus::tree::NodeContents::Matrix { matrix_type, target_count, source_count, .. } => {
                        println!("{}🔀 [{}] \"{}\" {} type={:?} t={:?} s={:?}", 
                            indent, path, id,
                            if desc.is_empty() { String::new() } else { format!("({})", desc) },
                            matrix_type, target_count, source_count);
                    }
                }

                child_numbers = guard.children().map(|c| c.read().number()).collect();
            }

            // Limit children display
            let limit = if child_numbers.len() > 15 && depth > 2 { 15 } else { child_numbers.len() };
            for &child_num in child_numbers.iter().take(limit) {
                let child_path = format!("{}.{}", path, child_num);
                Box::pin(explore_node_deep(client, &child_path, depth + 1, max_depth)).await;
            }
            if child_numbers.len() > limit {
                println!("{}... +{} more", indent, child_numbers.len() - limit);
            }
        }
        Ok(None) => {
            println!("{}❓ [{}] - not found", indent, path);
        }
        Err(e) => {
            println!("{}❌ [{}] - {}", indent, path, e);
        }
    }
}
