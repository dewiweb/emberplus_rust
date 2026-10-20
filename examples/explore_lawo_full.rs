//! Explore Lawo MC²36 with full tree expansion.
//!
//! Run with: cargo run --example explore_lawo_full

use ember_plus::{EmberClient, Result, GlowElement};
use tracing_subscriber::EnvFilter;

const LAWO_ADDRESS: &str = "192.168.100.199:9000";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("ember_plus=info".parse().unwrap()))
        .init();

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     LAWO MC²36 - Full Tree Exploration                     ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    println!("Connecting to {}...", LAWO_ADDRESS);
    let client = EmberClient::connect(LAWO_ADDRESS).await?;
    println!("✓ Connected!\n");

    // Get root and print it
    println!("Getting root directory...");
    let root = client.get_directory().await?;
    println!("Root has {} elements\n", root.elements.len());

    for elem in &root.elements {
        print_glow_element(elem, 0);
    }

    // Now explore specific paths by requesting their directory
    let paths_to_explore = vec![
        "28548",           // Channels
        "28548.28549",     // Inputs
    ];

    for path in paths_to_explore {
        println!("\n════════════════════════════════════════════════════════════");
        println!("  Expanding path: {}", path);
        println!("════════════════════════════════════════════════════════════\n");

        match client.expand(path).await {
            Ok(()) => {
                // Now read from tree cache
                let tree = client.tree().await;
                if let Ok(Some(node)) = tree.get_by_path_str(path) {
                    print_tree_node(&node, 0, 6);
                }
            }
            Err(e) => {
                println!("Error expanding {}: {}", path, e);
            }
        }
    }

    println!("\n════════════════════════════════════════════════════════════");
    println!("  Discovery Complete");
    println!("════════════════════════════════════════════════════════════");

    client.disconnect().await?;
    Ok(())
}

fn print_glow_element(elem: &GlowElement, depth: usize) {
    let indent = "  ".repeat(depth);
    match elem {
        GlowElement::Node(n) => {
            println!("{}📁 Node #{} id={:?} desc={:?}", indent, n.number, n.identifier, n.description);
            for child in &n.children {
                print_glow_element(child, depth + 1);
            }
        }
        GlowElement::QualifiedNode(path, n) => {
            println!("{}📁 QNode {:?} id={:?} desc={:?}", indent, path, n.identifier, n.description);
            for child in &n.children {
                print_glow_element(child, depth + 1);
            }
        }
        GlowElement::Parameter(p) => {
            println!("{}🎚️  Param #{} id={:?} value={:?} min={:?} max={:?}", 
                indent, p.number, p.identifier, p.value, p.minimum, p.maximum);
        }
        GlowElement::QualifiedParameter(path, p) => {
            println!("{}🎚️  QParam {:?} id={:?} value={:?}", indent, path, p.identifier, p.value);
        }
        GlowElement::Matrix(m) => {
            println!("{}🔀 Matrix #{} id={:?} type={:?}", indent, m.number, m.identifier, m.matrix_type);
        }
        GlowElement::QualifiedMatrix(path, m) => {
            println!("{}🔀 QMatrix {:?} id={:?}", indent, path, m.identifier);
        }
        GlowElement::Function(f) => {
            println!("{}⚡ Func #{} id={:?}", indent, f.number, f.identifier);
        }
        GlowElement::QualifiedFunction(path, f) => {
            println!("{}⚡ QFunc {:?} id={:?}", indent, path, f.identifier);
        }
        GlowElement::Template(t) => {
            println!("{}📋 Template #{}", indent, t.number);
        }
        GlowElement::Command(c) => {
            println!("{}⌘ Command {:?}", indent, c.number());
        }
    }
}

fn print_tree_node(node: &std::sync::Arc<parking_lot::RwLock<ember_plus::tree::TreeNode>>, depth: usize, max_depth: usize) {
    if depth > max_depth {
        println!("{}... (max depth)", "  ".repeat(depth));
        return;
    }

    let indent = "  ".repeat(depth);
    let guard = node.read();
    let id = guard.identifier().unwrap_or("?");
    let desc = guard.description().unwrap_or("");
    let contents = guard.contents();

    match contents {
        ember_plus::tree::NodeContents::Node { is_online, .. } => {
            println!("{}📁 #{} \"{}\" {} online={:?}", 
                indent, guard.number(), id, 
                if desc.is_empty() { String::new() } else { format!("({})", desc) },
                is_online);
        }
        ember_plus::tree::NodeContents::Parameter { value, minimum, maximum, access, stream_identifier, .. } => {
            println!("{}🎚️  #{} \"{}\" {}", indent, guard.number(), id,
                if desc.is_empty() { String::new() } else { format!("({})", desc) });
            println!("{}    value={:?} min={:?} max={:?} access={:?} stream={:?}", 
                indent, value, minimum, maximum, access, stream_identifier);
        }
        ember_plus::tree::NodeContents::Function { .. } => {
            println!("{}⚡ #{} \"{}\" {}", indent, guard.number(), id,
                if desc.is_empty() { String::new() } else { format!("({})", desc) });
        }
        ember_plus::tree::NodeContents::Matrix { matrix_type, target_count, source_count, .. } => {
            println!("{}🔀 #{} \"{}\" {} type={:?} t={:?} s={:?}", 
                indent, guard.number(), id,
                if desc.is_empty() { String::new() } else { format!("({})", desc) },
                matrix_type, target_count, source_count);
        }
    }

    let children: Vec<_> = guard.children().cloned().collect();
    drop(guard);

    let limit = if children.len() > 20 && depth > 2 { 20 } else { children.len() };
    for child in children.iter().take(limit) {
        print_tree_node(child, depth + 1, max_depth);
    }
    if children.len() > limit {
        println!("{}... +{} more children", indent, children.len() - limit);
    }
}
