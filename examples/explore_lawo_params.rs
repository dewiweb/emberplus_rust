//! Explore Lawo MC²36 actual parameters (fader, mute, meters).
//!
//! Run with: cargo run --example explore_lawo_params

use ember_plus::{EmberClient, Result, GlowElement};
use tracing_subscriber::EnvFilter;

const LAWO_ADDRESS: &str = "192.168.100.199:9000";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("ember_plus=info".parse().unwrap()))
        .init();

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     LAWO MC²36 - Parameter Discovery                       ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    println!("Connecting to {}...", LAWO_ADDRESS);
    let client = EmberClient::connect(LAWO_ADDRESS).await?;
    println!("✓ Connected!\n");

    // First, expand Inputs to get channel list
    println!("Expanding Inputs node...");
    client.expand("28548.28549").await?;
    
    let tree = client.tree().await;
    let inputs_node = tree.get_by_path_str("28548.28549")?;
    
    if let Some(inputs) = inputs_node {
        let channels: Vec<(i32, String)> = {
            let guard = inputs.read();
            guard.children().map(|c| {
                let cg = c.read();
                (cg.number(), cg.identifier().unwrap_or("?").to_string())
            }).collect()
        };
        drop(tree);

        println!("Found {} input channels\n", channels.len());

        // Take first channel for detailed exploration
        if let Some((ch_num, ch_id)) = channels.first() {
            let ch_path = format!("28548.28549.{}", ch_num);
            println!("════════════════════════════════════════════════════════════");
            println!("  Exploring Channel 1: {} (path: {})", ch_id, ch_path);
            println!("════════════════════════════════════════════════════════════\n");

            // Expand channel
            client.expand(&ch_path).await?;

            // Get the tree again
            let tree = client.tree().await;
            if let Ok(Some(ch_node)) = tree.get_by_path_str(&ch_path) {
                let sub_nodes: Vec<(i32, String, String)> = {
                    let guard = ch_node.read();
                    guard.children().map(|c| {
                        let cg = c.read();
                        (cg.number(), 
                         cg.identifier().unwrap_or("?").to_string(),
                         cg.description().unwrap_or("").to_string())
                    }).collect()
                };
                drop(tree);

                println!("Channel sub-nodes:");
                for (num, id, desc) in &sub_nodes {
                    println!("  - #{} \"{}\" ({})", num, id, desc);
                }

                // Now explore key nodes: Fader, Mute, Metering, Listen
                let key_nodes = vec![
                    ("Fader", "_400016b0"),
                    ("Mute", "_400016c0"),
                    ("Metering", "_40011970"),
                    ("Listen", "_3ec"),
                    ("Pan", "_400016d0"),
                    ("Channel States", "_40001030"),
                ];

                for (name, id) in key_nodes {
                    // Find the node number for this id
                    if let Some((num, _, _)) = sub_nodes.iter().find(|(_, i, _)| i == id) {
                        let path = format!("{}.{}", ch_path, num);
                        println!("\n════════════════════════════════════════════════════════════");
                        println!("  {} (path: {})", name, path);
                        println!("════════════════════════════════════════════════════════════\n");

                        // Expand this node
                        if let Err(e) = client.expand(&path).await {
                            println!("  Error expanding: {}", e);
                            continue;
                        }

                        // Print its contents
                        let tree = client.tree().await;
                        if let Ok(Some(node)) = tree.get_by_path_str(&path) {
                            print_node_recursive(&node, 0, 4);
                        }
                        drop(tree);
                    }
                }
            }
        }
    }

    println!("\n════════════════════════════════════════════════════════════");
    println!("  Discovery Complete");
    println!("════════════════════════════════════════════════════════════");

    client.disconnect().await?;
    Ok(())
}

fn print_node_recursive(
    node: &std::sync::Arc<parking_lot::RwLock<ember_plus::tree::TreeNode>>, 
    depth: usize, 
    max_depth: usize
) {
    if depth > max_depth {
        return;
    }

    let indent = "  ".repeat(depth);
    let guard = node.read();
    let num = guard.number();
    let id = guard.identifier().unwrap_or("?");
    let desc = guard.description().unwrap_or("");
    let contents = guard.contents();

    match contents {
        ember_plus::tree::NodeContents::Node { is_online, .. } => {
            println!("{}📁 #{} \"{}\" {} online={:?}", 
                indent, num, id, 
                if desc.is_empty() { String::new() } else { format!("({})", desc) },
                is_online);
        }
        ember_plus::tree::NodeContents::Parameter { 
            value, minimum, maximum, access, stream_identifier, 
            format, factor, parameter_type, ..
        } => {
            println!("{}🎚️  #{} \"{}\" {}", indent, num, id,
                if desc.is_empty() { String::new() } else { format!("({})", desc) });
            println!("{}    TYPE: {:?}", indent, parameter_type);
            println!("{}    VALUE: {:?}", indent, value);
            println!("{}    MIN: {:?}  MAX: {:?}", indent, minimum, maximum);
            println!("{}    ACCESS: {:?}  STREAM: {:?}", indent, access, stream_identifier);
            if format.is_some() || factor.is_some() {
                println!("{}    FORMAT: {:?}  FACTOR: {:?}", indent, format, factor);
            }
        }
        ember_plus::tree::NodeContents::Function { arguments, result, .. } => {
            println!("{}⚡ #{} \"{}\" {} (args={}, results={})", 
                indent, num, id,
                if desc.is_empty() { String::new() } else { format!("({})", desc) },
                arguments.len(), result.len());
        }
        ember_plus::tree::NodeContents::Matrix { matrix_type, target_count, source_count, .. } => {
            println!("{}🔀 #{} \"{}\" {} type={:?}", 
                indent, num, id,
                if desc.is_empty() { String::new() } else { format!("({})", desc) },
                matrix_type);
        }
    }

    let children: Vec<_> = guard.children().cloned().collect();
    drop(guard);

    for child in children.iter().take(30) {
        print_node_recursive(child, depth + 1, max_depth);
    }
    if children.len() > 30 {
        println!("{}... +{} more", indent, children.len() - 30);
    }
}
