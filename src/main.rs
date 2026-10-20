//! Ember+ CLI tool.

use ember_plus::{EmberClient, EmberServer, Result};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        println!("Ember+ CLI Tool");
        println!();
        println!("Usage:");
        println!("  {} client <host:port>     Connect to an Ember+ provider", args[0]);
        println!("  {} server <port>          Start an Ember+ provider", args[0]);
        println!();
        println!("Examples:");
        println!("  {} client 192.168.1.100:9000", args[0]);
        println!("  {} server 9000", args[0]);
        return Ok(());
    }

    match args[1].as_str() {
        "client" => {
            if args.len() < 3 {
                println!("Usage: {} client <host:port>", args[0]);
                return Ok(());
            }
            run_client(&args[2]).await?;
        }
        "server" => {
            let port = args.get(2).map(|s| s.as_str()).unwrap_or("9000");
            run_server(port).await?;
        }
        _ => {
            println!("Unknown command: {}", args[1]);
        }
    }

    Ok(())
}

async fn run_client(address: &str) -> Result<()> {
    println!("Connecting to {}...", address);
    
    let client = EmberClient::connect(address).await?;
    println!("Connected!");

    // Get root directory
    println!("\nGetting root directory...");
    let root = client.get_directory().await?;
    
    println!("\nTree structure:");
    print_elements(&root.elements, 0);

    // Keep connection open for a bit to receive updates
    println!("\nListening for updates (press Ctrl+C to exit)...");
    tokio::signal::ctrl_c().await.ok();

    client.disconnect().await?;
    println!("Disconnected.");

    Ok(())
}

fn print_elements(elements: &[ember_plus::GlowElement], indent: usize) {
    let prefix = "  ".repeat(indent);
    
    for element in elements {
        match element {
            ember_plus::GlowElement::Node(node) => {
                let id = node.identifier.as_deref().unwrap_or("(no id)");
                let desc = node.description.as_deref().unwrap_or("");
                println!("{}[{}] Node: {} {}", prefix, node.number, id, desc);
                print_elements(&node.children, indent + 1);
            }
            ember_plus::GlowElement::Parameter(param) => {
                let id = param.identifier.as_deref().unwrap_or("(no id)");
                let value = param.value.as_ref()
                    .map(|v| format!("{}", v))
                    .unwrap_or_default();
                println!("{}[{}] Parameter: {} = {}", prefix, param.number, id, value);
            }
            ember_plus::GlowElement::Function(func) => {
                let id = func.identifier.as_deref().unwrap_or("(no id)");
                println!("{}[{}] Function: {}", prefix, func.number, id);
            }
            ember_plus::GlowElement::Matrix(matrix) => {
                let id = matrix.identifier.as_deref().unwrap_or("(no id)");
                println!("{}[{}] Matrix: {}", prefix, matrix.number, id);
            }
            ember_plus::GlowElement::QualifiedNode(path, node) => {
                let id = node.identifier.as_deref().unwrap_or("(no id)");
                let path_str = path.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(".");
                println!("{}[{}] Node: {}", prefix, path_str, id);
                print_elements(&node.children, indent + 1);
            }
            ember_plus::GlowElement::QualifiedParameter(path, param) => {
                let id = param.identifier.as_deref().unwrap_or("(no id)");
                let value = param.value.as_ref()
                    .map(|v| format!("{}", v))
                    .unwrap_or_default();
                let path_str = path.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(".");
                println!("{}[{}] Parameter: {} = {}", prefix, path_str, id, value);
            }
            _ => {}
        }
    }
}

async fn run_server(port: &str) -> Result<()> {
    use ember_plus::tree::TreeNode;
    use ember_plus::EmberValue;

    let address = format!("0.0.0.0:{}", port);
    println!("Starting Ember+ server on {}...", address);

    let mut server = EmberServer::bind(&address).await?;

    // Create a sample tree
    let mut root = TreeNode::new_node(0);
    {
        let root_guard = root.contents_mut();
        if let ember_plus::tree::NodeContents::Node { identifier, description, is_root, .. } = root_guard {
            *identifier = Some("Root".to_string());
            *description = Some("Root node".to_string());
            *is_root = Some(true);
        }
    }

    // Add a parameter
    let param = TreeNode::new_parameter(1, EmberValue::Integer(42));
    root.add_child(std::sync::Arc::new(parking_lot::RwLock::new(param)));

    // Initialize the tree
    server.init(root).await;

    // Set up handlers
    server.set_value_handler(std::sync::Arc::new(|path, value| {
        println!("setValue at {:?}: {}", path, value);
        Ok(true)
    }));

    server.invoke_handler(std::sync::Arc::new(|path, id, args| {
        println!("invoke at {:?}, id={}, args={:?}", path, id, args);
        Ok(ember_plus::InvocationResult::success(id, vec![]))
    }));

    println!("Server running. Press Ctrl+C to stop.");
    
    // Run until interrupted
    server.run().await?;

    Ok(())
}
