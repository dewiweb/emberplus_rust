//! Example Ember+ server (provider).
//!
//! Run with: cargo run --example server -- [port]

use std::sync::Arc;
use ember_plus::{EmberServer, EmberValue, InvocationResult, Result, ParameterAccess};
use ember_plus::tree::{TreeNode, NodeContents};
use parking_lot::RwLock;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = std::env::args().collect();
    let port = args.get(1).map(|s| s.as_str()).unwrap_or("9000");
    let address = format!("0.0.0.0:{}", port);

    println!("Starting Ember+ server on {}...", address);
    let mut server = EmberServer::bind(&address).await?;

    // Build the tree
    let tree = build_sample_tree();
    server.init(tree).await;

    // Handle setValue operations
    server.set_value_handler(Arc::new(|path, value| {
        println!("setValue: path={:?}, value={}", path, value);
        Ok(true) // Accept the change
    }));

    // Handle function invocations
    server.invoke_handler(Arc::new(|path, invocation_id, args| {
        println!("invoke: path={:?}, id={}, args={:?}", path, invocation_id, args);
        
        // Return success with optional result values
        Ok(InvocationResult::success(invocation_id, vec![
            EmberValue::String("Function executed".to_string())
        ]))
    }));

    // Handle matrix operations
    server.matrix_handler(Arc::new(|path, connection| {
        println!("matrix: path={:?}, target={}, sources={:?}", 
            path, connection.target, connection.sources);
        Ok(())
    }));

    println!("Server ready! Connect with an Ember+ client.");
    println!("Press Ctrl+C to stop.");

    server.run().await?;

    Ok(())
}

fn build_sample_tree() -> TreeNode {
    // Root node
    let mut root = TreeNode::new_node(0);
    update_node_contents(&mut root, "Root", "Sample Ember+ Provider", true);

    // Audio node
    let mut audio = TreeNode::new_node(1);
    update_node_contents(&mut audio, "Audio", "Audio controls", false);

    // Master volume parameter
    let mut volume = TreeNode::new_parameter(1, EmberValue::Real(-6.0));
    update_parameter_contents(&mut volume, "MasterVolume", "Master volume in dB", 
        Some(EmberValue::Real(-96.0)), Some(EmberValue::Real(12.0)));
    audio.add_child(Arc::new(RwLock::new(volume)));

    // Mute parameter
    let mut mute = TreeNode::new_parameter(2, EmberValue::Boolean(false));
    update_parameter_contents(&mut mute, "Mute", "Master mute", None, None);
    audio.add_child(Arc::new(RwLock::new(mute)));

    root.add_child(Arc::new(RwLock::new(audio)));

    // Video node
    let mut video = TreeNode::new_node(2);
    update_node_contents(&mut video, "Video", "Video controls", false);

    // Brightness parameter
    let mut brightness = TreeNode::new_parameter(1, EmberValue::Integer(50));
    update_parameter_contents(&mut brightness, "Brightness", "Display brightness %",
        Some(EmberValue::Integer(0)), Some(EmberValue::Integer(100)));
    video.add_child(Arc::new(RwLock::new(brightness)));

    // Source parameter (enum)
    let mut source = TreeNode::new_parameter(2, EmberValue::Integer(0));
    update_parameter_contents(&mut source, "Source", "Video source selection", None, None);
    video.add_child(Arc::new(RwLock::new(source)));

    root.add_child(Arc::new(RwLock::new(video)));

    // System node
    let mut system = TreeNode::new_node(3);
    update_node_contents(&mut system, "System", "System information", false);

    // Device name parameter
    let mut name = TreeNode::new_parameter(1, EmberValue::String("Ember+ Test Device".to_string()));
    update_parameter_contents(&mut name, "DeviceName", "Device name", None, None);
    system.add_child(Arc::new(RwLock::new(name)));

    // Version parameter (read-only)
    let mut version = TreeNode::new_parameter(2, EmberValue::String("1.0.0".to_string()));
    update_parameter_contents(&mut version, "Version", "Firmware version", None, None);
    system.add_child(Arc::new(RwLock::new(version)));

    root.add_child(Arc::new(RwLock::new(system)));

    root
}

fn update_node_contents(node: &mut TreeNode, identifier: &str, description: &str, is_root: bool) {
    if let NodeContents::Node { 
        identifier: id, 
        description: desc, 
        is_root: root, 
        is_online,
        .. 
    } = node.contents_mut() {
        *id = Some(identifier.to_string());
        *desc = Some(description.to_string());
        *root = Some(is_root);
        *is_online = Some(true);
    }
}

fn update_parameter_contents(
    node: &mut TreeNode, 
    identifier: &str, 
    description: &str,
    min: Option<EmberValue>,
    max: Option<EmberValue>,
) {
    if let NodeContents::Parameter { 
        identifier: id, 
        description: desc,
        minimum,
        maximum,
        access,
        is_online,
        .. 
    } = node.contents_mut() {
        *id = Some(identifier.to_string());
        *desc = Some(description.to_string());
        *minimum = min;
        *maximum = max;
        *access = Some(ParameterAccess::ReadWrite);
        *is_online = Some(true);
    }
}
