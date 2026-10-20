//! Explore sub-nodes with full expansion to get identifiers.
//!
//! Run with: cargo run --example explore_lawo_subnodes

use ember_plus::{EmberClient, Result};
use tracing_subscriber::EnvFilter;

const LAWO_ADDRESS: &str = "192.168.100.199:9000";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("ember_plus=info".parse().unwrap()))
        .init();

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     LAWO MC²36 - Sub-node Identifier Discovery             ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let client = EmberClient::connect(LAWO_ADDRESS).await?;
    println!("✓ Connected\n");

    // Step 1: Get root and expand channels
    let _root = client.get_directory().await?;
    client.expand("28548").await?;
    client.expand("28548.28549").await?;

    // Get first channel number
    let first_ch = {
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

    let ch_num = first_ch.unwrap_or(0);
    if ch_num == 0 {
        println!("No channels found!");
        return Ok(());
    }

    let ch_path = format!("28548.28549.{}", ch_num);
    println!("Channel path: {}", ch_path);
    
    // Expand the channel
    client.expand(&ch_path).await?;

    // Get sub-node numbers
    let sub_nums: Vec<i32> = {
        let tree = client.tree().await;
        if let Ok(Some(ch)) = tree.get_by_path_str(&ch_path) {
            let guard = ch.read();
            guard.children().map(|c| c.read().number()).collect()
        } else {
            vec![]
        }
    };

    println!("Found {} sub-nodes. Expanding each...\n", sub_nums.len());

    // Expand each sub-node to get its identifier
    for &sub_num in &sub_nums {
        let sub_path = format!("{}.{}", ch_path, sub_num);
        
        // Expand this sub-node
        if let Err(e) = client.expand(&sub_path).await {
            println!("  #{}: Error expanding - {}", sub_num, e);
            continue;
        }

        // Now read its info
        let tree = client.tree().await;
        if let Ok(Some(node)) = tree.get_by_path_str(&sub_path) {
            let guard = node.read();
            let id = guard.identifier().unwrap_or("?");
            let desc = guard.description().unwrap_or("");
            let contents = guard.contents();

            let type_str = match contents {
                ember_plus::tree::NodeContents::Node { .. } => "NODE",
                ember_plus::tree::NodeContents::Parameter { .. } => "PARAM",
                ember_plus::tree::NodeContents::Function { .. } => "FUNC",
                ember_plus::tree::NodeContents::Matrix { .. } => "MATRIX",
            };

            println!("[{}] #{} \"{}\" - {}", type_str, sub_num, id, desc);

            // If it's the fader, mute, or metering node, explore deeper
            if id == "_400016b0" || id == "_400016c0" || id == "_40011970" || id == "_3ec" {
                println!("  ↳ Exploring {} parameters...", desc);
                
                let child_nums: Vec<i32> = guard.children().map(|c| c.read().number()).collect();
                drop(guard);
                drop(tree);

                for &child_num in child_nums.iter().take(10) {
                    let child_path = format!("{}.{}", sub_path, child_num);
                    if let Err(_) = client.expand(&child_path).await {
                        continue;
                    }

                    let tree2 = client.tree().await;
                    if let Ok(Some(child)) = tree2.get_by_path_str(&child_path) {
                        let cg = child.read();
                        let cid = cg.identifier().unwrap_or("?");
                        let cdesc = cg.description().unwrap_or("");
                        
                        if let ember_plus::tree::NodeContents::Parameter { 
                            value, minimum, maximum, access, parameter_type, ..
                        } = cg.contents() {
                            println!("    🎛️  #{} \"{}\" ({})", child_num, cid, cdesc);
                            println!("       PATH: {}", child_path);
                            println!("       TYPE: {:?}, ACCESS: {:?}", parameter_type, access);
                            println!("       VALUE: {:?}, RANGE: {:?} → {:?}", value, minimum, maximum);
                        } else {
                            println!("    📁 #{} \"{}\" ({})", child_num, cid, cdesc);
                        }
                    }
                }
            }
        }
    }

    println!("\n═══ Done ═══");
    client.disconnect().await?;
    Ok(())
}
