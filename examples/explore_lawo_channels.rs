//! Explore Lawo MC²36 Channels and control parameters.
//!
//! Run with: cargo run --example explore_lawo_channels

use ember_plus::{EmberClient, Result};
use tracing_subscriber::EnvFilter;

const LAWO_ADDRESS: &str = "192.168.100.199:9000";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("ember_plus=info".parse().unwrap()))
        .init();

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     LAWO MC²36 - Channel & Control Discovery               ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    println!("Connecting to {}...", LAWO_ADDRESS);
    let client = EmberClient::connect(LAWO_ADDRESS).await?;
    println!("✓ Connected!\n");

    // Known root paths from discovery

    // Explore Channels first (most likely to have faders)
    println!("════════════════════════════════════════════════════════════");
    println!("  EXPLORING: Channels (path: 28548)");
    println!("════════════════════════════════════════════════════════════\n");

    explore_node_deep(&client, "28548", 0, 4).await;

    // Explore Console node
    println!("\n════════════════════════════════════════════════════════════");
    println!("  EXPLORING: Console (path: 1991269)");
    println!("════════════════════════════════════════════════════════════\n");

    explore_node_deep(&client, "1991269", 0, 3).await;

    // Explore Monitoring Units (for meters)
    println!("\n════════════════════════════════════════════════════════════");
    println!("  EXPLORING: Monitoring Units (path: 1991112)");
    println!("════════════════════════════════════════════════════════════\n");

    explore_node_deep(&client, "1991112", 0, 3).await;

    // Explore Signals (for meters/levels)
    println!("\n════════════════════════════════════════════════════════════");
    println!("  EXPLORING: Signals (path: 671903)");
    println!("════════════════════════════════════════════════════════════\n");

    explore_node_deep(&client, "671903", 0, 3).await;

    // Explore identity
    println!("\n════════════════════════════════════════════════════════════");
    println!("  EXPLORING: identity (path: 1991262)");
    println!("════════════════════════════════════════════════════════════\n");

    explore_node_deep(&client, "1991262", 0, 3).await;

    println!("\n════════════════════════════════════════════════════════════");
    println!("  Discovery Complete");
    println!("════════════════════════════════════════════════════════════");

    client.disconnect().await?;
    Ok(())
}

async fn explore_node_deep(client: &EmberClient, path: &str, depth: usize, max_depth: usize) {
    if depth > max_depth {
        println!("{}... (max depth reached)", "  ".repeat(depth));
        return;
    }

    let indent = "  ".repeat(depth);

    // Try to get this element
    match client.get_element_by_path(path).await {
        Ok(Some(node)) => {
            let child_numbers: Vec<i32>;
            {
                let guard = node.read();
                let id = guard.identifier().unwrap_or("?");
                let desc = guard.description().unwrap_or("");
                let contents = guard.contents();

                // Print based on node type
                match contents {
                    ember_plus::tree::NodeContents::Node { is_online, .. } => {
                        println!("{}[NODE] {} \"{}\" {} online={:?}", 
                            indent, path, id, 
                            if desc.is_empty() { String::new() } else { format!("({})", desc) },
                            is_online);
                    }
                    ember_plus::tree::NodeContents::Parameter { value, minimum, maximum, access, stream_identifier, .. } => {
                        println!("{}[PARAM] {} \"{}\" {}", indent, path, id,
                            if desc.is_empty() { String::new() } else { format!("({})", desc) });
                        println!("{}        value={:?} min={:?} max={:?} access={:?} stream={:?}", 
                            indent, value, minimum, maximum, access, stream_identifier);
                    }
                    ember_plus::tree::NodeContents::Function { arguments, result, .. } => {
                        println!("{}[FUNC] {} \"{}\" {} args={} results={}", 
                            indent, path, id,
                            if desc.is_empty() { String::new() } else { format!("({})", desc) },
                            arguments.len(), result.len());
                    }
                    ember_plus::tree::NodeContents::Matrix { matrix_type, target_count, source_count, .. } => {
                        println!("{}[MATRIX] {} \"{}\" {} type={:?} targets={:?} sources={:?}", 
                            indent, path, id,
                            if desc.is_empty() { String::new() } else { format!("({})", desc) },
                            matrix_type, target_count, source_count);
                    }
                }

                child_numbers = guard.children().map(|c| c.read().number()).collect();
            }

            // Recursively explore children (limit to first 10 for large nodes)
            let limit = if child_numbers.len() > 10 && depth > 1 { 10 } else { child_numbers.len() };
            for &child_num in child_numbers.iter().take(limit) {
                let child_path = format!("{}.{}", path, child_num);
                Box::pin(explore_node_deep(client, &child_path, depth + 1, max_depth)).await;
            }
            if child_numbers.len() > limit {
                println!("{}... and {} more children", indent, child_numbers.len() - limit);
            }
        }
        Ok(None) => {
            println!("{}[?] {} - not found", indent, path);
        }
        Err(e) => {
            println!("{}[ERR] {} - {}", indent, path, e);
        }
    }
}
