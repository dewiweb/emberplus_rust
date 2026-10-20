//! Full Lawo MC²36 Ember+ tree discovery with optimized methods.
//!
//! Run with: cargo run --example discover_full

use ember_plus::{EmberClient, ChildInfo, ElementType, Result};
use std::collections::HashMap;
use tracing_subscriber::EnvFilter;

const LAWO_ADDRESS: &str = "192.168.100.199:9000";

#[derive(Debug, Default)]
struct DiscoveryStats {
    nodes: usize,
    parameters: usize,
    matrices: usize,
    functions: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("ember_plus=info".parse().unwrap()))
        .init();

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     LAWO MC²36 - Full Tree Discovery (Optimized)          ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    println!("Connecting to {}...", LAWO_ADDRESS);
    let client = EmberClient::connect(LAWO_ADDRESS).await?;
    println!("✓ Connected!\n");

    // Get root directory
    println!("Getting root directory...");
    let _ = client.get_directory().await?;

    // Discover root level
    let root_children = client.expand_with_info("").await.unwrap_or_default();
    println!("Root has {} top-level nodes\n", root_children.len());

    let mut stats = DiscoveryStats::default();
    let mut all_paths: HashMap<String, ChildInfo> = HashMap::new();

    // Explore each root node
    for child in &root_children {
        let path = format!("{}", child.number);
        println!("════════════════════════════════════════════════════════════");
        println!("  {} - {:?}", 
            child.identifier.as_deref().unwrap_or("?"),
            child.description.as_deref().unwrap_or(""));
        println!("════════════════════════════════════════════════════════════");

        // Deep explore this branch (2 levels)
        explore_branch(&client, &path, 0, 2, &mut stats, &mut all_paths).await;
    }

    // Summary
    println!("\n════════════════════════════════════════════════════════════");
    println!("  DISCOVERY COMPLETE");
    println!("════════════════════════════════════════════════════════════");
    println!("  Nodes:      {}", stats.nodes);
    println!("  Parameters: {}", stats.parameters);
    println!("  Matrices:   {}", stats.matrices);
    println!("  Functions:  {}", stats.functions);
    println!("  Total:      {}", all_paths.len());

    // Show matrices found
    let matrices: Vec<_> = all_paths.iter()
        .filter(|(_, info)| info.is_matrix())
        .collect();
    if !matrices.is_empty() {
        println!("\n  MATRICES FOUND:");
        for (path, info) in matrices {
            println!("    {} - {} ({}x{})", 
                path,
                info.identifier.as_deref().unwrap_or("?"),
                info.target_count.unwrap_or(0),
                info.source_count.unwrap_or(0));
        }
    }

    // Show functions found
    let functions: Vec<_> = all_paths.iter()
        .filter(|(_, info)| info.is_function())
        .collect();
    if !functions.is_empty() {
        println!("\n  FUNCTIONS FOUND:");
        for (path, info) in functions.iter().take(20) {
            println!("    {} - {}", 
                path,
                info.identifier.as_deref().unwrap_or("?"));
        }
        if functions.len() > 20 {
            println!("    ... and {} more", functions.len() - 20);
        }
    }

    client.disconnect().await?;
    Ok(())
}

async fn explore_branch(
    client: &EmberClient,
    path: &str,
    depth: usize,
    max_depth: usize,
    stats: &mut DiscoveryStats,
    all_paths: &mut HashMap<String, ChildInfo>,
) {
    if depth > max_depth {
        return;
    }

    let children = match client.expand_with_info(path).await {
        Ok(c) => c,
        Err(_) => return,
    };

    let indent = "  ".repeat(depth + 1);

    for child in &children {
        let child_path = if path.is_empty() {
            format!("{}", child.number)
        } else {
            format!("{}.{}", path, child.number)
        };

        // Update stats
        match child.element_type {
            ElementType::Node => stats.nodes += 1,
            ElementType::Parameter => stats.parameters += 1,
            ElementType::Matrix => stats.matrices += 1,
            ElementType::Function => stats.functions += 1,
        }

        all_paths.insert(child_path.clone(), child.clone());

        // Print info
        let type_icon = match child.element_type {
            ElementType::Node => "📁",
            ElementType::Parameter => "🎚️",
            ElementType::Matrix => "🔀",
            ElementType::Function => "⚡",
        };

        let id = child.identifier.as_deref().unwrap_or("?");
        let desc = child.description.as_deref().unwrap_or("");

        if depth < 2 {
            println!("{}{} {} {}", indent, type_icon, id, 
                if desc.is_empty() { String::new() } else { format!("({})", desc) });
        }

        // Recurse for nodes
        if child.is_node() && depth < max_depth {
            Box::pin(explore_branch(client, &child_path, depth + 1, max_depth, stats, all_paths)).await;
        }
    }
}
