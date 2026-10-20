//! Explore exact parameters inside Fader/Mute/Metering nodes.
//!
//! Run with: cargo run --example explore_lawo_parameters

use ember_plus::{EmberClient, Result};
use tracing_subscriber::EnvFilter;

const LAWO_ADDRESS: &str = "192.168.100.199:9000";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("ember_plus=info".parse().unwrap()))
        .init();

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     LAWO MC²36 - Deep Parameter Exploration                ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let client = EmberClient::connect(LAWO_ADDRESS).await?;
    println!("✓ Connected to {}\n", LAWO_ADDRESS);

    // Step 1: Expand to get channel list
    println!("═══ Expanding Inputs ═══");
    client.expand("28548").await?;
    client.expand("28548.28549").await?;

    // Get first channel
    let first_channel = {
        let tree = client.tree().await;
        if let Ok(Some(inputs)) = tree.get_by_path_str("28548.28549") {
            let guard = inputs.read();
            let result = guard.children().next().map(|c| c.read().number());
            drop(guard);
            result
        } else {
            None
        }
    };

    let ch_num = match first_channel {
        Some(n) => n,
        None => {
            println!("No channels found!");
            return Ok(());
        }
    };

    let ch_path = format!("28548.28549.{}", ch_num);
    println!("First channel path: {}\n", ch_path);

    // Step 2: Expand channel and find sub-nodes
    client.expand(&ch_path).await?;

    let sub_nodes: Vec<(i32, String, String)> = {
        let tree = client.tree().await;
        if let Ok(Some(ch)) = tree.get_by_path_str(&ch_path) {
            let guard = ch.read();
            guard.children().map(|c| {
                let cg = c.read();
                (cg.number(), 
                 cg.identifier().unwrap_or("?").to_string(),
                 cg.description().unwrap_or("").to_string())
            }).collect()
        } else {
            vec![]
        }
    };

    println!("Channel sub-nodes:");
    for (num, id, desc) in &sub_nodes {
        println!("  #{} \"{}\" - {}", num, id, desc);
    }

    // Step 3: Find and deeply explore Fader, Mute, Metering
    let targets = vec![
        ("_400016b0", "Fader"),
        ("_400016c0", "Mute"),
        ("_40011970", "Metering"),
        ("_3ec", "Listen/PFL"),
    ];

    for (target_id, target_name) in targets {
        if let Some((num, _, _)) = sub_nodes.iter().find(|(_, id, _)| id == target_id) {
            let node_path = format!("{}.{}", ch_path, num);
            println!("\n╔════════════════════════════════════════════════════════════╗");
            println!("║  {} (path: {})", target_name, node_path);
            println!("╚════════════════════════════════════════════════════════════╝\n");

            // Expand this node
            if let Err(e) = client.expand(&node_path).await {
                println!("  Error expanding: {}", e);
                continue;
            }

            // Print all parameters inside
            print_node_params(&client, &node_path, 0, 4).await;
        } else {
            println!("\n{} ({}) not found in channel", target_name, target_id);
        }
    }

    println!("\n═══ Discovery Complete ═══");
    client.disconnect().await?;
    Ok(())
}

async fn print_node_params(client: &EmberClient, path: &str, depth: usize, max_depth: usize) {
    if depth > max_depth {
        return;
    }

    let indent = "  ".repeat(depth);
    let tree = client.tree().await;
    
    if let Ok(Some(node)) = tree.get_by_path_str(path) {
        let guard = node.read();
        let num = guard.number();
        let id = guard.identifier().unwrap_or("?");
        let desc = guard.description().unwrap_or("");
        let contents = guard.contents();

        match contents {
            ember_plus::tree::NodeContents::Node { is_online, .. } => {
                println!("{}📁 #{} \"{}\" {} [online={:?}]", indent, num, id, 
                    if desc.is_empty() { "".to_string() } else { format!("({})", desc) },
                    is_online);
            }
            ember_plus::tree::NodeContents::Parameter { 
                value, minimum, maximum, access, stream_identifier, 
                parameter_type, format, factor, ..
            } => {
                println!("{}🎛️  #{} \"{}\" {}", indent, num, id,
                    if desc.is_empty() { "".to_string() } else { format!("({})", desc) });
                println!("{}    ├─ PATH: {}", indent, path);
                println!("{}    ├─ TYPE: {:?}", indent, parameter_type);
                println!("{}    ├─ VALUE: {:?}", indent, value);
                println!("{}    ├─ RANGE: {:?} → {:?}", indent, minimum, maximum);
                println!("{}    ├─ ACCESS: {:?}", indent, access);
                if stream_identifier.is_some() {
                    println!("{}    ├─ STREAM: {:?}", indent, stream_identifier);
                }
                if format.is_some() || factor.is_some() {
                    println!("{}    └─ FORMAT: {:?}, FACTOR: {:?}", indent, format, factor);
                }
            }
            ember_plus::tree::NodeContents::Function { .. } => {
                println!("{}⚡ #{} \"{}\" (function)", indent, num, id);
            }
            ember_plus::tree::NodeContents::Matrix { .. } => {
                println!("{}🔀 #{} \"{}\" (matrix)", indent, num, id);
            }
        }

        let children: Vec<i32> = guard.children().map(|c| c.read().number()).collect();
        drop(guard);
        drop(tree);

        // Expand and recurse into children
        for child_num in children.iter().take(20) {
            let child_path = format!("{}.{}", path, child_num);
            if let Err(_) = client.expand(&child_path).await {
                continue;
            }
            Box::pin(print_node_params(client, &child_path, depth + 1, max_depth)).await;
        }
        if children.len() > 20 {
            println!("{}... +{} more children", indent, children.len() - 20);
        }
    }
}
