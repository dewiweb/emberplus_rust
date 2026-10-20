//! Lazy discovery for Lawo MC²36 - load categories first, channels on demand
//! Now with PARALLEL loading of all categories!
//!
//! Run with: cargo run --example lazy_discovery

use ember_plus::{EmberClient, ChildInfo, Result};
use futures_util::future::join_all;
use std::collections::HashMap;
use std::time::Instant;

const LAWO_ADDRESS: &str = "192.168.100.199:9000";

/// Category info with path and channel count
#[derive(Debug, Clone)]
pub struct Category {
    pub name: String,
    pub path: String,
    pub channel_count: usize,
    pub channels_loaded: bool,
}

/// Console structure with lazy-loaded categories
#[derive(Debug)]
pub struct ConsoleStructure {
    pub categories: HashMap<String, Category>,
    pub sapphire_path: String,
}

impl ConsoleStructure {
    pub fn new() -> Self {
        Self {
            categories: HashMap::new(),
            sapphire_path: String::new(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     LAWO MC²36 - Lazy Discovery                            ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let total_start = Instant::now();

    // Connect
    println!("1. Connecting...");
    let client = EmberClient::connect(LAWO_ADDRESS).await?;
    println!("   ✓ Connected");

    // Get root
    let _ = client.get_directory().await?;

    // Phase 1: Discover categories (fast)
    println!("\n2. Discovering categories...");
    let start = Instant::now();
    let structure = discover_categories(&client).await?;
    println!("   ✓ Categories found in {:?}", start.elapsed());

    // Display categories
    println!("\n   ┌─────────────────────────────────────────────────────────┐");
    println!("   │  CATEGORIES                                             │");
    println!("   ├─────────────────────────────────────────────────────────┤");
    for (key, cat) in &structure.categories {
        println!("   │  {:12} │ {:30} │ {:4} ch │", key, cat.name, cat.channel_count);
    }
    println!("   └─────────────────────────────────────────────────────────┘");

    // Phase 2: Load ALL categories with PARALLEL CONNECTIONS
    println!("\n3. Loading ALL categories with PARALLEL CONNECTIONS...");
    let start = Instant::now();
    
    let cat_keys = ["Sums", "Auxes", "Masters", "Groups", "Monitors", "ListenSums"];
    let cat_paths: Vec<(String, String)> = cat_keys.iter()
        .filter_map(|k| structure.categories.get(*k).map(|c| (k.to_string(), c.path.clone())))
        .collect();
    
    // Each category gets its own connection for TRUE parallelism
    let futures: Vec<_> = cat_paths.iter()
        .map(|(key, path)| {
            let key = key.clone();
            let path = path.clone();
            async move {
                let client = EmberClient::connect(LAWO_ADDRESS).await.ok()?;
                let _ = client.get_directory().await.ok()?;
                let channels = client.expand_with_info(&path).await.unwrap_or_default();
                let _ = client.disconnect().await;
                Some((key, channels))
            }
        })
        .collect();
    
    // Execute ALL connections in parallel
    let results = join_all(futures).await;
    
    println!("   ✓ All categories loaded in {:?}", start.elapsed());
    
    // Display results
    for result in results.iter().flatten() {
        let (key, channels) = result;
        let cat_name = structure.categories.get(key).map(|c| c.name.as_str()).unwrap_or(key);
        println!("   {:12} : {:3} channels", cat_name, channels.len());
    }

    let total = total_start.elapsed();
    println!("\n════════════════════════════════════════════════════════════");
    println!("  TOTAL TIME: {:?}", total);
    println!("════════════════════════════════════════════════════════════");

    client.disconnect().await?;
    Ok(())
}

/// Discover categories WITHOUT loading channels (fast)
async fn discover_categories(client: &EmberClient) -> Result<ConsoleStructure> {
    let mut structure = ConsoleStructure::new();

    // Get root children to find Sapphire (_2)
    let root_children = client.expand_with_info("").await.unwrap_or_default();
    
    // Find Sapphire node (identifier _2)
    let sapphire = root_children.iter()
        .find(|c| c.identifier.as_deref() == Some("_2"));
    
    let sapphire_num = match sapphire {
        Some(s) => s.number,
        None => {
            println!("   Warning: Sapphire node not found, using default 28548");
            28548
        }
    };
    
    structure.sapphire_path = format!("{}", sapphire_num);
    
    // Get Sapphire children (these are the categories) - just one expand
    let sapphire_children = client.expand_with_info(&structure.sapphire_path).await.unwrap_or_default();
    
    // Map of category descriptions to keys
    let category_keys = [
        ("Inputs", "Inputs"),
        ("Sums", "Sums"),
        ("Auxes", "Auxes"),
        ("Masters", "Masters"),
        ("Groups", "Groups"),
        ("Monitors", "Monitors"),
        ("Listen Sums", "ListenSums"),
        ("GP Channels", "GPChannels"),
        ("Special Sums", "SpecialSums"),
        ("Surround Masters", "SurroundMasters"),
    ];
    
    // Just register categories WITHOUT loading their channels
    for child in &sapphire_children {
        let desc = child.description.as_deref().unwrap_or("");
        
        for (cat_desc, key) in &category_keys {
            if desc == *cat_desc {
                let cat_path = format!("{}.{}", structure.sapphire_path, child.number);
                
                // Don't load channels here - just register the category
                structure.categories.insert(key.to_string(), Category {
                    name: desc.to_string(),
                    path: cat_path,
                    channel_count: 0, // Will be updated when loaded
                    channels_loaded: false,
                });
                break;
            }
        }
    }
    
    Ok(structure)
}

/// Load channels for a specific category (limited to max_channels)
/// Fast version - no fallback, just expand_with_info
async fn load_category_channels(
    client: &EmberClient,
    category_path: &str,
    max_channels: usize,
) -> Result<Vec<ChildInfo>> {
    let channels = client.expand_with_info(category_path).await.unwrap_or_default();
    Ok(channels.into_iter().take(max_channels).collect())
}
