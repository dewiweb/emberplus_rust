//! Benchmark discovery time for Lawo MC²36 input channels
//!
//! Run with: cargo run --example bench_discovery

use ember_plus::{EmberClient, Result};
use std::time::Instant;

const LAWO_ADDRESS: &str = "192.168.100.199:9000";

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     LAWO MC²36 - Discovery Benchmark                       ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let total_start = Instant::now();

    // Connect
    println!("1. Connecting...");
    let start = Instant::now();
    let client = EmberClient::connect(LAWO_ADDRESS).await?;
    println!("   ✓ Connected in {:?}", start.elapsed());

    // Get root directory
    println!("2. Getting root directory...");
    let start = Instant::now();
    let _ = client.get_directory().await?;
    println!("   ✓ Root in {:?}", start.elapsed());

    // Find Sapphire node (should be _2)
    println!("3. Finding Sapphire/Inputs...");
    let start = Instant::now();
    let root_children = client.expand_with_info("").await.unwrap_or_default();
    
    // Find _2 (Sapphire)
    let sapphire = root_children.iter().find(|c| c.identifier.as_deref() == Some("_2"));
    let sapphire_num = sapphire.map(|s| s.number).unwrap_or(28548);
    println!("   Sapphire node: {}", sapphire_num);
    
    // Expand Sapphire to find Inputs
    let sapphire_path = format!("{}", sapphire_num);
    let sapphire_children = client.expand_with_info(&sapphire_path).await.unwrap_or_default();
    
    // Find Inputs node
    let inputs = sapphire_children.iter().find(|c| c.description.as_deref() == Some("Inputs"));
    let inputs_num = inputs.map(|i| i.number).unwrap_or(28549);
    let inputs_path = format!("{}.{}", sapphire_num, inputs_num);
    println!("   Inputs path: {}", inputs_path);
    println!("   ✓ Structure found in {:?}", start.elapsed());

    // Expand Inputs to get channels
    println!("4. Getting channels...");
    let start = Instant::now();
    let channels = client.expand_with_info(&inputs_path).await.unwrap_or_default();
    let channel_numbers: Vec<i32> = channels.iter().map(|c| c.number).collect();
    println!("   ✓ Found {} channels in {:?}", channel_numbers.len(), start.elapsed());
    
    if channel_numbers.is_empty() {
        println!("   No channels found");
        client.disconnect().await?;
        return Ok(());
    }

    // Batch expand channels
    println!("5. Batch expanding {} channels...", channel_numbers.len().min(24));
    let start = Instant::now();
    let ch_paths: Vec<String> = channel_numbers.iter()
        .take(24)
        .map(|n| format!("{}.{}", inputs_path, n))
        .collect();
    let ch_refs: Vec<&str> = ch_paths.iter().map(|s: &String| s.as_str()).collect();
    let _ = client.expand_batch(&ch_refs).await;
    println!("   ✓ Channels expanded in {:?}", start.elapsed());

    // Discover faders and mutes
    println!("6. Discovering fader/mute parameters...");
    let start = Instant::now();
    let mut discovered = 0;

    for (idx, ch_num) in channel_numbers.iter().take(24).enumerate() {
        let ch_path = format!("{}.{}", inputs_path, ch_num);
        
        // Get children with identifiers
        if let Ok(children) = client.expand_with_info(&ch_path).await {
            for child in &children {
                let child_id = child.identifier.as_deref().unwrap_or("");
                let node_path = format!("{}.{}", ch_path, child.number);
                
                // Fader node (_400016b0)
                if child_id == "_400016b0" {
                    if let Ok(fader_children) = client.expand_with_info(&node_path).await {
                        if fader_children.first().is_some() {
                            discovered += 1;
                        }
                    }
                }
                
                // Mute node (_400016c0)
                if child_id == "_400016c0" {
                    if let Ok(mute_children) = client.expand_with_info(&node_path).await {
                        if mute_children.first().is_some() {
                            discovered += 1;
                        }
                    }
                }
            }
        }
        
        if (idx + 1) % 6 == 0 {
            print!(".");
        }
    }
    println!("\n   ✓ Parameters discovered in {:?}", start.elapsed());
    println!("   Found {} parameters (faders + mutes)", discovered);

    let total = total_start.elapsed();
    println!("\n════════════════════════════════════════════════════════════");
    println!("  TOTAL DISCOVERY TIME: {:?}", total);
    println!("════════════════════════════════════════════════════════════");

    client.disconnect().await?;
    Ok(())
}
