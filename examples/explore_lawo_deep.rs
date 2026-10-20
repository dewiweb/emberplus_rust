//! Deep exploration of Lawo MC²36 channel parameters.
//!
//! Run with: cargo run --example explore_lawo_deep

use ember_plus::{EmberClient, Result};
use tracing_subscriber::EnvFilter;

const LAWO_ADDRESS: &str = "192.168.100.199:9000";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("ember_plus=info".parse().unwrap()))
        .init();

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     LAWO MC²36 - Deep Parameter Discovery                  ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    println!("Connecting to {}...", LAWO_ADDRESS);
    let client = EmberClient::connect(LAWO_ADDRESS).await?;
    println!("✓ Connected!\n");

    // Get root first
    let root = client.get_directory().await?;
    println!("Root: {} elements", root.elements.len());

    // Step by step expansion to discover structure
    println!("\n═══ Step 1: Expand Channels (28548) ═══");
    client.expand("28548").await?;
    print_tree_at_path(&client, "28548", 2).await;

    println!("\n═══ Step 2: Expand Inputs (28548.28549) ═══");
    client.expand("28548.28549").await?;
    
    // Get first channel number
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

    if let Some(ch_num) = first_channel {
        let ch_path = format!("28548.28549.{}", ch_num);
        println!("\n═══ Step 3: Expand First Channel ({}) ═══", ch_path);
        client.expand(&ch_path).await?;
        print_tree_at_path(&client, &ch_path, 2).await;

        // Get children of channel to find Fader, Mute, etc.
        let channel_children: Vec<(i32, String, String)> = {
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

        println!("\nChannel has {} sub-nodes:", channel_children.len());
        for (num, id, desc) in &channel_children {
            println!("  #{}: \"{}\" - {}", num, id, desc);
        }

        // Now expand each child to see its parameters
        for (num, _id, desc) in &channel_children {
            let sub_path = format!("{}.{}", ch_path, num);
            println!("\n═══ Expanding: {} ({}) ═══", desc, sub_path);
            
            if let Err(e) = client.expand(&sub_path).await {
                println!("  Error: {}", e);
                continue;
            }

            print_tree_at_path(&client, &sub_path, 3).await;
        }
    }

    println!("\n════════════════════════════════════════════════════════════");
    println!("  Discovery Complete");
    println!("════════════════════════════════════════════════════════════");

    client.disconnect().await?;
    Ok(())
}

async fn print_tree_at_path(client: &EmberClient, path: &str, max_depth: usize) {
    let tree = client.tree().await;
    if let Ok(Some(node)) = tree.get_by_path_str(path) {
        print_node(&node, 0, max_depth);
    } else {
        println!("  Path not found: {}", path);
    }
}

fn print_node(
    node: &std::sync::Arc<parking_lot::RwLock<ember_plus::tree::TreeNode>>, 
    depth: usize, 
    max_depth: usize
) {
    let indent = "  ".repeat(depth);
    let guard = node.read();
    let num = guard.number();
    let id = guard.identifier().unwrap_or("?");
    let desc = guard.description().unwrap_or("");
    let contents = guard.contents();

    match contents {
        ember_plus::tree::NodeContents::Node { is_online, .. } => {
            let online_str = is_online.map(|b| if b { "✓" } else { "✗" }).unwrap_or("?");
            println!("{}📁 #{} \"{}\" {} [{}]", indent, num, id, 
                if desc.is_empty() { String::new() } else { format!("({})", desc) }, online_str);
        }
        ember_plus::tree::NodeContents::Parameter { 
            value, minimum, maximum, access, stream_identifier, parameter_type, ..
        } => {
            println!("{}🎛️  #{} \"{}\" {}", indent, num, id,
                if desc.is_empty() { String::new() } else { format!("({})", desc) });
            println!("{}   ├─ type={:?} access={:?}", indent, parameter_type, access);
            println!("{}   ├─ value={:?}", indent, value);
            if minimum.is_some() || maximum.is_some() {
                println!("{}   ├─ min={:?} max={:?}", indent, minimum, maximum);
            }
            if stream_identifier.is_some() {
                println!("{}   └─ stream={:?}", indent, stream_identifier);
            }
        }
        ember_plus::tree::NodeContents::Function { .. } => {
            println!("{}⚡ #{} \"{}\" {}", indent, num, id,
                if desc.is_empty() { String::new() } else { format!("({})", desc) });
        }
        ember_plus::tree::NodeContents::Matrix { matrix_type, .. } => {
            println!("{}🔀 #{} \"{}\" {} type={:?}", indent, num, id,
                if desc.is_empty() { String::new() } else { format!("({})", desc) }, matrix_type);
        }
    }

    if depth < max_depth {
        let children: Vec<_> = guard.children().cloned().collect();
        drop(guard);

        for child in children.iter().take(25) {
            print_node(child, depth + 1, max_depth);
        }
        if children.len() > 25 {
            println!("{}... +{} more children", indent, children.len() - 25);
        }
    }
}
