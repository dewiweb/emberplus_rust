//! Server listener and connection handling.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock, Mutex};
use tracing::{debug, error, info, warn};

use crate::error::{Error, Result};
use crate::codec;
use crate::glow::{
    GlowRoot, GlowElement, GlowNode, GlowParameter, GlowFunction, 
    GlowMatrix, GlowConnection, GlowCommand, EmberPath, EmberValue,
};
use crate::s101::{S101Codec, S101Encoder, S101Message};
use crate::tree::{EmberTree, TreeNode, TreeNodeRef};
use super::handler::{
    SetValueHandler, InvokeHandler, MatrixHandler, 
    default_set_value_handler, default_invoke_handler, default_matrix_handler
};

/// Configuration for the Ember+ server.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Keep-alive interval in seconds
    pub keepalive_interval_secs: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            max_connections: 100,
            keepalive_interval_secs: 5,
        }
    }
}

/// Client connection state.
struct ClientConnection {
    addr: SocketAddr,
    subscriptions: Vec<EmberPath>,
}

/// An Ember+ server (provider).
pub struct EmberServer {
    /// Listener address
    address: String,
    /// Configuration
    config: ServerConfig,
    /// The Ember+ tree
    tree: Arc<RwLock<EmberTree>>,
    /// Connected clients
    clients: Arc<RwLock<HashMap<SocketAddr, ClientConnection>>>,
    /// Shutdown signal sender
    shutdown_tx: Option<mpsc::Sender<()>>,
    /// SetValue handler
    on_set_value: SetValueHandler,
    /// Invoke handler
    on_invoke: InvokeHandler,
    /// Matrix handler
    on_matrix: MatrixHandler,
}

impl EmberServer {
    /// Create a new server bound to the given address.
    pub async fn bind(address: &str) -> Result<Self> {
        Self::bind_with_config(address, ServerConfig::default()).await
    }

    /// Create a new server with custom configuration.
    pub async fn bind_with_config(address: &str, config: ServerConfig) -> Result<Self> {
        Ok(EmberServer {
            address: address.to_string(),
            config,
            tree: Arc::new(RwLock::new(EmberTree::new())),
            clients: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: None,
            on_set_value: default_set_value_handler(),
            on_invoke: default_invoke_handler(),
            on_matrix: default_matrix_handler(),
        })
    }

    /// Set the setValue handler.
    pub fn set_value_handler(&mut self, handler: SetValueHandler) {
        self.on_set_value = handler;
    }

    /// Set the invoke handler.
    pub fn invoke_handler(&mut self, handler: InvokeHandler) {
        self.on_invoke = handler;
    }

    /// Set the matrix handler.
    pub fn matrix_handler(&mut self, handler: MatrixHandler) {
        self.on_matrix = handler;
    }

    /// Initialize the server with a tree.
    pub async fn init(&self, root: TreeNode) {
        let mut tree = self.tree.write().await;
        tree.add_root_child(root);
    }

    /// Add a root-level node to the tree.
    pub async fn add_root(&self, node: TreeNode) -> TreeNodeRef {
        self.tree.write().await.add_root_child(node)
    }

    /// Get the tree for modification.
    pub async fn tree(&self) -> tokio::sync::RwLockWriteGuard<'_, EmberTree> {
        self.tree.write().await
    }

    /// Update a parameter value and notify subscribers.
    pub async fn update(&self, path: &EmberPath, value: EmberValue) -> Result<()> {
        // Update tree
        {
            let tree = self.tree.read().await;
            if let Some(node) = tree.get_by_path(path) {
                node.write().set_value(value.clone());
            }
        }

        // Notify subscribed clients
        self.notify_subscribers(path, &value).await?;

        Ok(())
    }

    /// Notify subscribers of a value change.
    async fn notify_subscribers(&self, path: &EmberPath, value: &EmberValue) -> Result<()> {
        // Build update message
        let mut param = GlowParameter::new(*path.last().unwrap_or(&0));
        param.value = Some(value.clone());
        
        let root = GlowRoot::with_elements(vec![
            GlowElement::QualifiedParameter(path.clone(), param)
        ]);

        let glow_bytes = codec::encode(&root)?;
        let frame_bytes = S101Encoder::encode_ember_packet(&glow_bytes);

        // TODO: Send to subscribed clients
        // This would require maintaining client connection handles

        Ok(())
    }

    /// Run the server.
    pub async fn run(&mut self) -> Result<()> {
        let listener = TcpListener::bind(&self.address).await.map_err(Error::Io)?;
        info!("Ember+ server listening on {}", self.address);

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let tree = Arc::clone(&self.tree);
        let clients = Arc::clone(&self.clients);
        let on_set_value = Arc::clone(&self.on_set_value);
        let on_invoke = Arc::clone(&self.on_invoke);
        let on_matrix = Arc::clone(&self.on_matrix);
        let max_connections = self.config.max_connections;

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            let client_count = clients.read().await.len();
                            if client_count >= max_connections {
                                warn!("Max connections reached, rejecting {}", addr);
                                continue;
                            }

                            info!("New connection from {}", addr);

                            let tree = Arc::clone(&tree);
                            let clients = Arc::clone(&clients);
                            let on_set_value = Arc::clone(&on_set_value);
                            let on_invoke = Arc::clone(&on_invoke);
                            let on_matrix = Arc::clone(&on_matrix);

                            tokio::spawn(async move {
                                if let Err(e) = handle_client(
                                    stream, addr, tree, clients,
                                    on_set_value, on_invoke, on_matrix
                                ).await {
                                    error!("Client {} error: {}", addr, e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Accept error: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Server shutting down");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Shutdown the server.
    pub async fn shutdown(&self) {
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(()).await;
        }
    }
}

/// Handle a client connection.
async fn handle_client(
    mut stream: TcpStream,
    addr: SocketAddr,
    tree: Arc<RwLock<EmberTree>>,
    clients: Arc<RwLock<HashMap<SocketAddr, ClientConnection>>>,
    on_set_value: SetValueHandler,
    on_invoke: InvokeHandler,
    on_matrix: MatrixHandler,
) -> Result<()> {
    stream.set_nodelay(true).ok();

    // Register client
    clients.write().await.insert(addr, ClientConnection {
        addr,
        subscriptions: Vec::new(),
    });

    let mut codec = S101Codec::new();
    let mut buffer = vec![0u8; 65536];

    loop {
        let n = match stream.read(&mut buffer).await {
            Ok(0) => {
                info!("Client {} disconnected", addr);
                break;
            }
            Ok(n) => n,
            Err(e) => {
                error!("Read error from {}: {}", addr, e);
                break;
            }
        };

        debug!("Received {} bytes from {}", n, addr);
        codec.feed(&buffer[..n]);

        // Process frames
        while let Ok(Some(frame)) = codec.decode_frame() {
            let message = frame.to_message();

            match message {
                S101Message::EmberData { payload, .. } => {
                    if let Ok(request) = codec::decode(&payload) {
                        let response = process_request(
                            &request, &tree, &clients, addr,
                            &on_set_value, &on_invoke, &on_matrix
                        ).await?;

                        if !response.is_empty() {
                            let glow_bytes = codec::encode(&response)?;
                            let frame_bytes = S101Encoder::encode_ember_packet(&glow_bytes);
                            stream.write_all(&frame_bytes).await.map_err(Error::Io)?;
                        }
                    }
                }
                S101Message::KeepAliveRequest => {
                    let response = S101Encoder::encode_keepalive_response();
                    stream.write_all(&response).await.map_err(Error::Io)?;
                }
                S101Message::KeepAliveResponse => {
                    debug!("Keep-alive from {}", addr);
                }
            }
        }
    }

    // Unregister client
    clients.write().await.remove(&addr);

    Ok(())
}

/// Process a client request.
async fn process_request(
    request: &GlowRoot,
    tree: &Arc<RwLock<EmberTree>>,
    clients: &Arc<RwLock<HashMap<SocketAddr, ClientConnection>>>,
    addr: SocketAddr,
    on_set_value: &SetValueHandler,
    on_invoke: &InvokeHandler,
    on_matrix: &MatrixHandler,
) -> Result<GlowRoot> {
    let mut response = GlowRoot::new();

    for element in &request.elements {
        process_element(
            element, vec![], tree, clients, addr,
            on_set_value, on_invoke, on_matrix, &mut response
        ).await?;
    }

    Ok(response)
}

/// Process a single element from a request.
async fn process_element(
    element: &GlowElement,
    parent_path: Vec<i32>,
    tree: &Arc<RwLock<EmberTree>>,
    clients: &Arc<RwLock<HashMap<SocketAddr, ClientConnection>>>,
    addr: SocketAddr,
    on_set_value: &SetValueHandler,
    on_invoke: &InvokeHandler,
    on_matrix: &MatrixHandler,
    response: &mut GlowRoot,
) -> Result<()> {
    match element {
        GlowElement::Command(cmd) => {
            match cmd {
                GlowCommand::GetDirectory => {
                    // Return children at path
                    let tree_guard = tree.read().await;
                    
                    if parent_path.is_empty() {
                        // Return root children
                        for child in tree_guard.children() {
                            response.elements.push(child.read().to_glow_element());
                        }
                    } else if let Some(node) = tree_guard.get_by_path(&parent_path) {
                        let node_guard = node.read();
                        for child in node_guard.children() {
                            response.elements.push(child.read().to_glow_element());
                        }
                    }
                }
                GlowCommand::Subscribe => {
                    let mut clients_guard = clients.write().await;
                    if let Some(client) = clients_guard.get_mut(&addr) {
                        if !client.subscriptions.contains(&parent_path) {
                            client.subscriptions.push(parent_path.clone());
                        }
                    }
                }
                GlowCommand::Unsubscribe => {
                    let mut clients_guard = clients.write().await;
                    if let Some(client) = clients_guard.get_mut(&addr) {
                        client.subscriptions.retain(|p| p != &parent_path);
                    }
                }
                GlowCommand::Invoke { invocation_id, arguments } => {
                    let result = on_invoke(&parent_path, *invocation_id, arguments)?;
                    response.invocation_results.push(result);
                }
            }
        }
        GlowElement::Node(node) => {
            let mut path = parent_path;
            path.push(node.number);

            for child in &node.children {
                Box::pin(process_element(
                    child, path.clone(), tree, clients, addr,
                    on_set_value, on_invoke, on_matrix, response
                )).await?;
            }
        }
        GlowElement::Parameter(param) => {
            let mut path = parent_path;
            path.push(param.number);

            // Handle setValue
            if let Some(value) = &param.value {
                if on_set_value(&path, value)? {
                    // Update tree
                    let tree_guard = tree.read().await;
                    if let Some(node) = tree_guard.get_by_path(&path) {
                        node.write().set_value(value.clone());
                    }
                }
            }
        }
        GlowElement::Function(func) => {
            let mut path = parent_path;
            path.push(func.number);

            for child in &func.children {
                Box::pin(process_element(
                    child, path.clone(), tree, clients, addr,
                    on_set_value, on_invoke, on_matrix, response
                )).await?;
            }
        }
        GlowElement::Matrix(matrix) => {
            let mut path = parent_path;
            path.push(matrix.number);

            // Handle connection operations
            for conn in &matrix.connections {
                on_matrix(&path, conn)?;
            }
        }
        GlowElement::QualifiedNode(path, node) => {
            for child in &node.children {
                Box::pin(process_element(
                    child, path.clone(), tree, clients, addr,
                    on_set_value, on_invoke, on_matrix, response
                )).await?;
            }
        }
        GlowElement::QualifiedParameter(path, param) => {
            if let Some(value) = &param.value {
                if on_set_value(path, value)? {
                    let tree_guard = tree.read().await;
                    if let Some(node) = tree_guard.get_by_path(path) {
                        node.write().set_value(value.clone());
                    }
                }
            }
        }
        _ => {}
    }

    Ok(())
}
