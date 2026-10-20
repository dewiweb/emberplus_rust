//! Client connection handling.

use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use crate::error::{Error, Result};
use crate::codec;
use crate::glow::{
    GlowRoot, GlowElement, EmberPath,
    InvocationResult, parse_path, EmberValue, CommandBuilder,
};
use crate::s101::{S101Codec, S101Encoder, S101Message};
use crate::tree::{EmberTree, TreeNodeRef};

/// Configuration for the Ember+ client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Request timeout
    pub request_timeout: Duration,
    /// Keep-alive interval
    pub keepalive_interval: Duration,
    /// Auto-reconnect on disconnect
    pub auto_reconnect: bool,
    /// Maximum reconnect attempts
    pub max_reconnect_attempts: u32,
}

impl Default for ClientConfig {
    fn default() -> Self {
        ClientConfig {
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
            keepalive_interval: Duration::from_secs(5),
            auto_reconnect: true,
            max_reconnect_attempts: 3,
        }
    }
}

/// Callback type for value updates.
pub type ValueCallback = Box<dyn Fn(&EmberPath, &EmberValue) + Send + Sync>;

/// Type of Ember+ element
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementType {
    /// Node (container)
    Node,
    /// Parameter (value)
    Parameter,
    /// Matrix
    Matrix,
    /// Function
    Function,
}

/// Information about a child element returned by expand_with_info.
#[derive(Debug, Clone)]
pub struct ChildInfo {
    /// Element number
    pub number: i32,
    /// Element identifier (e.g. "_400016b0")
    pub identifier: Option<String>,
    /// Element description (e.g. "Fader")
    pub description: Option<String>,
    /// Element type
    pub element_type: ElementType,
    /// Parameter value if available
    pub value: Option<EmberValue>,
    /// For parameters: minimum value
    pub minimum: Option<EmberValue>,
    /// For parameters: maximum value
    pub maximum: Option<EmberValue>,
    /// For matrix: number of targets
    pub target_count: Option<i32>,
    /// For matrix: number of sources
    pub source_count: Option<i32>,
}

impl ChildInfo {
    /// Check if this is a parameter
    pub fn is_parameter(&self) -> bool {
        self.element_type == ElementType::Parameter
    }
    
    /// Check if this is a node
    pub fn is_node(&self) -> bool {
        self.element_type == ElementType::Node
    }
    
    /// Check if this is a matrix
    pub fn is_matrix(&self) -> bool {
        self.element_type == ElementType::Matrix
    }
    
    /// Check if this is a function
    pub fn is_function(&self) -> bool {
        self.element_type == ElementType::Function
    }
}

/// Callback type for stream updates.
pub type StreamCallback = Box<dyn Fn(i32, &EmberValue) + Send + Sync>;

/// An Ember+ client for connecting to providers.
pub struct EmberClient {
    /// Remote address
    address: String,
    /// Configuration
    config: ClientConfig,
    /// TCP write half (for sending)
    write_half: Arc<Mutex<Option<tokio::net::tcp::OwnedWriteHalf>>>,
    /// S101 codec
    #[allow(dead_code)]
    codec: Arc<Mutex<S101Codec>>,
    /// Local tree cache
    tree: Arc<RwLock<EmberTree>>,
    /// Connected flag
    connected: Arc<RwLock<bool>>,
    /// Request counter for invocations
    request_counter: Arc<Mutex<i32>>,
    /// Value update callbacks
    value_callbacks: Arc<RwLock<Vec<ValueCallback>>>,
    /// Stream update callbacks  
    stream_callbacks: Arc<RwLock<Vec<StreamCallback>>>,
    /// Mapping from streamIdentifier to path (for stream -> value callback conversion)
    stream_to_path: Arc<RwLock<std::collections::HashMap<i32, Vec<i32>>>>,
    /// Channel to receive data from background reader
    rx: Arc<Mutex<Option<mpsc::Receiver<BackgroundMessage>>>>,
    /// Handle to background reader task
    #[allow(dead_code)]
    reader_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

/// Messages from background reader
#[derive(Debug)]
pub enum BackgroundMessage {
    /// Glow data received
    GlowData(GlowRoot),
    /// Connection closed
    Disconnected,
}

impl EmberClient {
    /// Connect to an Ember+ provider.
    pub async fn connect(address: &str) -> Result<Self> {
        Self::connect_with_config(address, ClientConfig::default()).await
    }

    /// Connect with custom configuration.
    pub async fn connect_with_config(address: &str, config: ClientConfig) -> Result<Self> {
        info!("Connecting to Ember+ provider at {}", address);

        let stream = timeout(config.connect_timeout, TcpStream::connect(address))
            .await
            .map_err(|_| Error::Timeout)?
            .map_err(Error::Io)?;

        stream.set_nodelay(true).ok();

        info!("Connected to {}", address);

        // Split stream into reader and writer to avoid lock contention
        let (read_half, write_half) = stream.into_split();
        let read_half = Arc::new(Mutex::new(Some(read_half)));
        let write_half = Arc::new(Mutex::new(Some(write_half)));
        
        let codec = Arc::new(Mutex::new(S101Codec::new()));
        let connected = Arc::new(RwLock::new(true));
        let tree = Arc::new(RwLock::new(EmberTree::new()));
        
        // Create channel for background reader to send data
        let (tx, rx) = mpsc::channel::<BackgroundMessage>(1000);
        
        // Create callback storage and stream mapping
        let value_callbacks: Arc<RwLock<Vec<ValueCallback>>> = Arc::new(RwLock::new(Vec::new()));
        let stream_callbacks: Arc<RwLock<Vec<StreamCallback>>> = Arc::new(RwLock::new(Vec::new()));
        let stream_to_path: Arc<RwLock<std::collections::HashMap<i32, Vec<i32>>>> = Arc::new(RwLock::new(std::collections::HashMap::new()));
        
        // Clone Arcs for background task
        let bg_read = read_half.clone();
        let bg_write = write_half.clone();
        let bg_codec = codec.clone();
        let bg_connected = connected.clone();
        let bg_tree = tree.clone();
        let bg_value_callbacks = value_callbacks.clone();
        let bg_stream_callbacks = stream_callbacks.clone();
        let bg_stream_to_path = stream_to_path.clone();
        
        // Spawn background reader task
        let reader_handle = tokio::spawn(async move {
            Self::background_reader(bg_read, bg_write, bg_codec, bg_connected, bg_tree, tx, bg_value_callbacks, bg_stream_callbacks, bg_stream_to_path).await;
        });

        let client = EmberClient {
            address: address.to_string(),
            config,
            write_half,
            codec,
            tree,
            connected,
            request_counter: Arc::new(Mutex::new(0)),
            value_callbacks,
            stream_callbacks,
            stream_to_path,
            rx: Arc::new(Mutex::new(Some(rx))),
            reader_handle: Arc::new(Mutex::new(Some(reader_handle))),
        };

        Ok(client)
    }

    /// Background reader task that handles keep-alive and incoming data.
    async fn background_reader(
        read_half: Arc<Mutex<Option<tokio::net::tcp::OwnedReadHalf>>>,
        write_half: Arc<Mutex<Option<tokio::net::tcp::OwnedWriteHalf>>>,
        codec: Arc<Mutex<S101Codec>>,
        connected: Arc<RwLock<bool>>,
        tree: Arc<RwLock<EmberTree>>,
        tx: mpsc::Sender<BackgroundMessage>,
        value_callbacks: Arc<RwLock<Vec<ValueCallback>>>,
        stream_callbacks: Arc<RwLock<Vec<StreamCallback>>>,
        stream_to_path: Arc<RwLock<std::collections::HashMap<i32, Vec<i32>>>>,
    ) {
        let mut buffer = vec![0u8; 65536];
        // Accumulator for multi-packet messages
        let mut accumulated_payload: Vec<u8> = Vec::new();
        
        let mut loop_count = 0u64;
        loop {
            loop_count += 1;
            
            // Check if still connected
            if !*connected.read().await {
                debug!("Background reader: disconnected flag set, exiting after {} loops", loop_count);
                break;
            }
            
            // Read with timeout to allow checking connected flag
            let read_result = {
                let mut read_guard = read_half.lock().await;
                if let Some(ref mut r) = *read_guard {
                    match timeout(Duration::from_secs(1), r.read(&mut buffer)).await {
                        Ok(Ok(n)) => Some(n),
                        Ok(Err(e)) => {
                            error!("Background reader: read error: {}", e);
                            None
                        }
                        Err(_) => {
                            // Timeout, continue loop to check connected flag
                            continue;
                        }
                    }
                } else {
                    None
                }
            };
            
            let n = match read_result {
                Some(0) => {
                    debug!("Background reader: connection closed by server");
                    *connected.write().await = false;
                    let _ = tx.try_send(BackgroundMessage::Disconnected);
                    break;
                }
                Some(n) => n,
                None => {
                    *connected.write().await = false;
                    let _ = tx.try_send(BackgroundMessage::Disconnected);
                    break;
                }
            };
            
            debug!("Background reader: received {} bytes", n);
            
            // Feed to decoder
            let mut codec_guard = codec.lock().await;
            codec_guard.feed(&buffer[..n]);
            
            // Process all complete frames
            while let Ok(Some(frame)) = codec_guard.decode_frame() {
                let message = frame.to_message();
                
                match message {
                    S101Message::KeepAliveRequest => {
                        // Respond to keep-alive using write_half (no contention with reader)
                        let response = S101Encoder::encode_keepalive_response();
                        let mut write_guard = write_half.lock().await;
                        if let Some(ref mut w) = *write_guard {
                            let _ = w.write_all(&response).await;
                            let _ = w.flush().await;
                        }
                        debug!("Background reader: sent keep-alive response");
                    }
                    S101Message::KeepAliveResponse => {
                        debug!("Background reader: received keep-alive response");
                    }
                    S101Message::EmberData { payload, flags, .. } => {
                        debug!("EmberData received: {} bytes, first={}, last={}", payload.len(), flags.first_packet, flags.last_packet);
                        // Handle multi-packet messages
                        if flags.first_packet {
                            accumulated_payload.clear();
                        }
                        accumulated_payload.extend(&payload);
                        
                        // Only decode when we have the last packet
                        if flags.last_packet {
                        let decode_result = crate::codec::decode(&accumulated_payload);
                        if decode_result.is_err() {
                            eprintln!("[EMBER DECODE ERROR] {:?}", decode_result.as_ref().unwrap_err());
                        }
                        if let Ok(root) = decode_result {
                                debug!("Decoded: {} elements, {} streams", root.elements.len(), root.streams.len());
                                tree.write().await.update_from_glow(&root);
                                
                                // Notify callbacks for value updates (important for subscriptions!)
                                // Callbacks handle ALL real-time data (streams + parameter changes)
                                Self::notify_callbacks_static(&root, &value_callbacks, &stream_callbacks, &stream_to_path).await;
                                
                                // Only send to command channel if this message has elements
                                // (i.e., it's a command response, not just stream data).
                                // Stream-only messages are already handled by callbacks above.
                                // This prevents high-frequency meter streams from flooding
                                // the channel and blocking command responses.
                                if !root.elements.is_empty() || !root.invocation_results.is_empty() {
                                    if tx.try_send(BackgroundMessage::GlowData(root)).is_err() {
                                        warn!("Background reader: command channel full, dropping response");
                                    }
                                }
                            } else {
                                warn!("Background reader: failed to decode Glow message");
                            }
                            accumulated_payload.clear();
                        } else {
                            debug!("Background reader: accumulated {} bytes (waiting for more packets)", accumulated_payload.len());
                        }
                    }
                }
            }
        }
        
        debug!("Background reader: exiting");
    }

    /// Static version of notify callbacks for use in background reader
    async fn notify_callbacks_static(
        root: &GlowRoot,
        value_callbacks: &Arc<RwLock<Vec<ValueCallback>>>,
        stream_callbacks: &Arc<RwLock<Vec<StreamCallback>>>,
        stream_to_path: &Arc<RwLock<std::collections::HashMap<i32, Vec<i32>>>>,
    ) {
        // Notify stream callbacks
        let stream_cbs = stream_callbacks.read().await;
        for entry in &root.streams {
            for callback in stream_cbs.iter() {
                callback(entry.stream_identifier, &entry.value);
            }
        }
        drop(stream_cbs);

        // Also notify value callbacks for stream updates using stream_to_path mapping
        if !root.streams.is_empty() {
            let value_cbs = value_callbacks.read().await;
            let path_map = stream_to_path.read().await;
            debug!("Processing {} streams, path_map has {} entries", root.streams.len(), path_map.len());
            for entry in &root.streams {
                if let Some(path) = path_map.get(&entry.stream_identifier) {
                    debug!("Stream {} -> path {:?}", entry.stream_identifier, path);
                    for callback in value_cbs.iter() {
                        callback(path, &entry.value);
                    }
                } else {
                    debug!("Stream {} has no path mapping (normal during subscription transitions)", entry.stream_identifier);
                }
            }
        }

        // Notify value callbacks for parameter updates
        let value_cbs = value_callbacks.read().await;
        for element in &root.elements {
            Self::notify_element_static(element, vec![], &value_cbs);
        }
    }

    /// Static recursive notification for elements
    fn notify_element_static(
        element: &GlowElement,
        parent_path: Vec<i32>,
        callbacks: &[ValueCallback],
    ) {
        match element {
            GlowElement::Parameter(param) => {
                if let Some(value) = &param.value {
                    let mut path = parent_path;
                    path.push(param.number);
                    for callback in callbacks {
                        callback(&path, value);
                    }
                }
            }
            GlowElement::QualifiedParameter(path, param) => {
                if let Some(value) = &param.value {
                    for callback in callbacks {
                        callback(path, value);
                    }
                }
            }
            GlowElement::Node(node) => {
                let mut path = parent_path;
                path.push(node.number);
                for child in &node.children {
                    Self::notify_element_static(child, path.clone(), callbacks);
                }
            }
            GlowElement::QualifiedNode(path, node) => {
                for child in &node.children {
                    Self::notify_element_static(child, path.clone(), callbacks);
                }
            }
            _ => {}
        }
    }

    /// Check if connected.
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Disconnect from the provider.
    pub async fn disconnect(&self) -> Result<()> {
        info!("Disconnecting from {}", self.address);
        
        *self.connected.write().await = false;
        
        if let Some(writer) = self.write_half.lock().await.take() {
            drop(writer);
        }

        Ok(())
    }

    /// Get the root directory.
    pub async fn get_directory(&self) -> Result<GlowRoot> {
        let root = GlowRoot::get_directory();
        self.send_and_receive(root).await
    }

    /// Send a raw Glow request and get response.
    pub async fn send_request(&self, request: GlowRoot) -> Result<GlowRoot> {
        self.send_and_receive(request).await
    }

    /// Get an element by path (numeric path like "0.1.2").
    /// This navigates progressively through the tree, requesting children at each level.
    pub async fn get_element_by_path(&self, path: &str) -> Result<Option<TreeNodeRef>> {
        let path_vec = parse_path(path)?;
        
        // Check if already in cache
        if let Some(node) = self.tree.read().await.get_by_path(&path_vec) {
            return Ok(Some(node));
        }

        // Navigate progressively: for each level, ensure we have the children
        for i in 0..path_vec.len() {
            let current_path = &path_vec[0..=i];
            
            // Check if this level is in cache
            if self.tree.read().await.get_by_path(current_path).is_some() {
                continue;
            }

            // Get parent path and identifier
            let parent_path = if i == 0 { vec![] } else { path_vec[0..i].to_vec() };
            
            // Get identifier from parent's children info (if available)
            let identifier = if !parent_path.is_empty() {
                let tree = self.tree.read().await;
                tree.get_by_path(&parent_path)
                    .and_then(|parent| {
                        parent.read().children()
                            .find(|c| c.read().number() == current_path[i])
                            .and_then(|c| c.read().identifier().map(|s| s.to_string()))
                    })
            } else {
                // For root children, check root nodes
                let tree = self.tree.read().await;
                tree.get_by_path(current_path)
                    .and_then(|n| n.read().identifier().map(|s| s.to_string()))
            };

            // Request this level with identifier
            let elements = CommandBuilder::get_directory_at_path_with_info(
                &current_path.to_vec(),
                identifier.as_deref()
            );
            let root = GlowRoot::with_elements(elements);
            let response = self.send_and_receive(root).await?;
            self.tree.write().await.update_from_glow(&response);
        }

        // Return from cache
        Ok(self.tree.read().await.get_by_path(&path_vec))
    }

    /// Expand the tree under a node (get all children recursively).
    pub async fn expand(&self, path: &str) -> Result<()> {
        let path_vec = parse_path(path)?;
        self.expand_path(&path_vec).await
    }

    /// Expand starting from root.
    pub async fn expand_root(&self) -> Result<()> {
        // Get root directory first
        let root_response = self.get_directory().await?;
        self.tree.write().await.update_from_glow(&root_response);

        // Expand each root child
        for element in &root_response.elements {
            if let Some(number) = element.number() {
                self.expand_path(&[number]).await?;
            }
        }

        Ok(())
    }

    /// Expand a specific path (single level - children only).
    async fn expand_path(&self, path: &[i32]) -> Result<()> {
        let path_vec = path.to_vec();
        let elements = CommandBuilder::get_directory_at_path(&path_vec);
        let root = GlowRoot::with_elements(elements);
        let response = self.send_and_receive(root).await?;

        self.tree.write().await.update_from_glow(&response);
        Ok(())
    }

    /// Expand multiple paths in parallel (batch expansion).
    /// Returns after all expansions complete.
    pub async fn expand_batch(&self, paths: &[&str]) -> Result<()> {
        use futures_util::future::join_all;
        
        let futures: Vec<_> = paths.iter()
            .map(|p| self.expand(p))
            .collect();
        
        let results = join_all(futures).await;
        
        // Return first error if any
        for result in results {
            result?;
        }
        
        Ok(())
    }

    /// Expand a path and return child info (number, identifier, description).
    /// More efficient than expand + separate tree queries.
    pub async fn expand_with_info(&self, path: &str) -> Result<Vec<ChildInfo>> {
        let path_vec = parse_path(path)?;
        
        // Send expand request
        let elements = CommandBuilder::get_directory_at_path(&path_vec);
        let root = GlowRoot::with_elements(elements);
        let response = self.send_and_receive(root).await?;
        
        // Update tree
        self.tree.write().await.update_from_glow(&response);
        
        // Extract child info from response
        let mut children = Vec::new();
        self.extract_children_info(&response.elements, &path_vec, &mut children);
        
        Ok(children)
    }

    /// Extract child information from Glow elements.
    fn extract_children_info(&self, elements: &[GlowElement], parent_path: &[i32], out: &mut Vec<ChildInfo>) {
        for element in elements {
            match element {
                GlowElement::Node(node) => {
                    out.push(ChildInfo {
                        number: node.number,
                        identifier: node.identifier.clone(),
                        description: node.description.clone(),
                        element_type: ElementType::Node,
                        value: None,
                        minimum: None,
                        maximum: None,
                        target_count: None,
                        source_count: None,
                    });
                    // Also extract children of this node
                    if !node.children.is_empty() {
                        let child_path: Vec<i32> = parent_path.iter().copied().chain(std::iter::once(node.number)).collect();
                        self.extract_children_info(&node.children, &child_path, out);
                    }
                }
                GlowElement::Parameter(param) => {
                    out.push(ChildInfo {
                        number: param.number,
                        identifier: param.identifier.clone(),
                        description: param.description.clone(),
                        element_type: ElementType::Parameter,
                        value: param.value.clone(),
                        minimum: param.minimum.clone(),
                        maximum: param.maximum.clone(),
                        target_count: None,
                        source_count: None,
                    });
                }
                GlowElement::Matrix(matrix) => {
                    out.push(ChildInfo {
                        number: matrix.number,
                        identifier: matrix.identifier.clone(),
                        description: matrix.description.clone(),
                        element_type: ElementType::Matrix,
                        value: None,
                        minimum: None,
                        maximum: None,
                        target_count: matrix.target_count,
                        source_count: matrix.source_count,
                    });
                }
                GlowElement::Function(func) => {
                    out.push(ChildInfo {
                        number: func.number,
                        identifier: func.identifier.clone(),
                        description: func.description.clone(),
                        element_type: ElementType::Function,
                        value: None,
                        minimum: None,
                        maximum: None,
                        target_count: None,
                        source_count: None,
                    });
                }
                GlowElement::QualifiedNode(path, node) => {
                    if path.len() == parent_path.len() + 1 && path.starts_with(parent_path) {
                        out.push(ChildInfo {
                            number: *path.last().unwrap_or(&0),
                            identifier: node.identifier.clone(),
                            description: node.description.clone(),
                            element_type: ElementType::Node,
                            value: None,
                            minimum: None,
                            maximum: None,
                            target_count: None,
                            source_count: None,
                        });
                    }
                    self.extract_children_info(&node.children, parent_path, out);
                }
                GlowElement::QualifiedParameter(path, param) => {
                    if path.len() == parent_path.len() + 1 && path.starts_with(parent_path) {
                        out.push(ChildInfo {
                            number: *path.last().unwrap_or(&0),
                            identifier: param.identifier.clone(),
                            description: param.description.clone(),
                            element_type: ElementType::Parameter,
                            value: param.value.clone(),
                            minimum: param.minimum.clone(),
                            maximum: param.maximum.clone(),
                            target_count: None,
                            source_count: None,
                        });
                    }
                }
                GlowElement::QualifiedMatrix(path, matrix) => {
                    if path.len() == parent_path.len() + 1 && path.starts_with(parent_path) {
                        out.push(ChildInfo {
                            number: *path.last().unwrap_or(&0),
                            identifier: matrix.identifier.clone(),
                            description: matrix.description.clone(),
                            element_type: ElementType::Matrix,
                            value: None,
                            minimum: None,
                            maximum: None,
                            target_count: matrix.target_count,
                            source_count: matrix.source_count,
                        });
                    }
                }
                GlowElement::QualifiedFunction(path, func) => {
                    if path.len() == parent_path.len() + 1 && path.starts_with(parent_path) {
                        out.push(ChildInfo {
                            number: *path.last().unwrap_or(&0),
                            identifier: func.identifier.clone(),
                            description: func.description.clone(),
                            element_type: ElementType::Function,
                            value: None,
                            minimum: None,
                            maximum: None,
                            target_count: None,
                            source_count: None,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    /// Deep expand: expand a path and all its children recursively up to max_depth.
    pub async fn expand_deep(&self, path: &str, max_depth: u32) -> Result<()> {
        self.expand_deep_internal(path, 0, max_depth).await
    }

    async fn expand_deep_internal(&self, path: &str, current_depth: u32, max_depth: u32) -> Result<()> {
        if current_depth >= max_depth {
            return Ok(());
        }

        // Expand this level
        let children = self.expand_with_info(path).await?;
        
        // Collect child paths that are nodes (not parameters)
        let child_paths: Vec<String> = children.iter()
            .filter(|c| c.is_node())
            .map(|c| format!("{}.{}", path, c.number))
            .collect();

        // Expand children in parallel
        if !child_paths.is_empty() {
            use futures_util::future::join_all;
            
            let futures: Vec<_> = child_paths.iter()
                .map(|p| self.expand_deep_internal(p, current_depth + 1, max_depth))
                .collect();
            
            let _ = join_all(futures).await;
        }

        Ok(())
    }

    /// Find a child node by identifier under a parent path.
    pub async fn find_child_by_id(&self, parent_path: &str, identifier: &str) -> Result<Option<(i32, String)>> {
        // Use expand_with_info instead of relying on local tree
        let children = self.expand_with_info(parent_path).await?;
        
        for child in children {
            if child.identifier.as_deref() == Some(identifier) {
                let child_path = if parent_path.is_empty() {
                    format!("{}", child.number)
                } else {
                    format!("{}.{}", parent_path, child.number)
                };
                return Ok(Some((child.number, child_path)));
            }
        }
        
        Ok(None)
    }

    /// Navigate to a node by a path of identifiers (e.g., "_2/_1/_1/_400016b0/_400016b1")
    /// Returns the numeric path and the final ChildInfo if found.
    pub async fn navigate_by_identifiers(&self, id_path: &str) -> Result<Option<(String, ChildInfo)>> {
        let identifiers: Vec<&str> = id_path.split('/').filter(|s| !s.is_empty()).collect();
        
        if identifiers.is_empty() {
            return Ok(None);
        }
        
        let mut current_path = String::new();
        let mut last_info: Option<ChildInfo> = None;
        
        for identifier in identifiers {
            let children = self.expand_with_info(&current_path).await?;
            
            let found = children.into_iter().find(|c| c.identifier.as_deref() == Some(identifier));
            
            if let Some(child) = found {
                current_path = if current_path.is_empty() {
                    format!("{}", child.number)
                } else {
                    format!("{}.{}", current_path, child.number)
                };
                last_info = Some(child);
            } else {
                return Ok(None);
            }
        }
        
        Ok(last_info.map(|info| (current_path, info)))
    }

    /// Subscribe to multiple paths at once.
    pub async fn subscribe_batch(&self, paths: &[&str]) -> Result<()> {
        use futures_util::future::join_all;
        
        let futures: Vec<_> = paths.iter()
            .map(|p| self.subscribe(p))
            .collect();
        
        let results = join_all(futures).await;
        
        for result in results {
            result?;
        }
        
        Ok(())
    }

    /// Recursively expand an element's children.
    #[allow(dead_code)]
    async fn expand_element(&self, element: &GlowElement, parent_path: Vec<i32>) -> Result<()> {
        match element {
            GlowElement::Node(node) => {
                let mut path = parent_path;
                path.push(node.number);

                // Request children
                let elements = CommandBuilder::get_directory_at_path(&path);
                let root = GlowRoot::with_elements(elements);
                if let Ok(response) = self.send_and_receive(root).await {
                    self.tree.write().await.update_from_glow(&response);

                    for child in &node.children {
                        Box::pin(self.expand_element(child, path.clone())).await?;
                    }
                }
            }
            GlowElement::QualifiedNode(path, node) => {
                let elements = CommandBuilder::get_directory_at_path(path);
                let root = GlowRoot::with_elements(elements);
                if let Ok(response) = self.send_and_receive(root).await {
                    self.tree.write().await.update_from_glow(&response);

                    for child in &node.children {
                        Box::pin(self.expand_element(child, path.clone())).await?;
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Set a parameter value.
    pub async fn set_value(&self, path: &str, value: EmberValue) -> Result<()> {
        let path_vec = parse_path(path)?;
        let elements = CommandBuilder::set_value_at_path(&path_vec, value);
        let root = GlowRoot::with_elements(elements);
        
        self.send_and_receive(root).await?;
        Ok(())
    }

    /// Subscribe to changes at a path.
    /// Note: This is fire-and-forget - values arrive asynchronously via callbacks.
    pub async fn subscribe(&self, path: &str) -> Result<()> {
        let path_vec = parse_path(path)?;
        
        // Register the mapping from streamIdentifier (last path element) to full path
        // This allows us to route stream updates to value callbacks
        if let Some(&stream_id) = path_vec.last() {
            self.stream_to_path.write().await.insert(stream_id, path_vec.clone());
            debug!("Registered stream mapping: {} -> {:?}", stream_id, path_vec);
        }
        
        let elements = CommandBuilder::subscribe_at_path(&path_vec);
        let root = GlowRoot::with_elements(elements);
        
        // Use send_only since subscriptions don't get immediate responses
        // Values will arrive asynchronously via the value_change callbacks
        self.send_only(root).await
    }

    /// Subscribe to changes at multiple paths in a single TCP packet.
    /// Much faster than calling subscribe() for each path individually
    /// (1 mutex lock + 1 encode + 1 TCP write vs N of each).
    pub async fn subscribe_many(&self, paths: &[&str]) -> Result<()> {
        if paths.is_empty() {
            return Ok(());
        }
        
        let mut path_vecs = Vec::with_capacity(paths.len());
        {
            let mut stream_map = self.stream_to_path.write().await;
            for path_str in paths {
                let path_vec = parse_path(path_str)?;
                if let Some(&stream_id) = path_vec.last() {
                    stream_map.insert(stream_id, path_vec.clone());
                }
                path_vecs.push(path_vec);
            }
        }
        
        let elements = CommandBuilder::subscribe_many_paths(&path_vecs);
        let root = GlowRoot::with_elements(elements);
        info!("subscribe_many: {} paths in 1 packet ({} elements)", paths.len(), root.elements.len());
        self.send_only(root).await
    }

    /// Unsubscribe from changes at multiple paths in a single TCP packet.
    pub async fn unsubscribe_many(&self, paths: &[&str]) -> Result<()> {
        if paths.is_empty() {
            return Ok(());
        }
        
        let mut path_vecs = Vec::with_capacity(paths.len());
        {
            let mut stream_map = self.stream_to_path.write().await;
            for path_str in paths {
                let path_vec = parse_path(path_str)?;
                if let Some(&stream_id) = path_vec.last() {
                    stream_map.remove(&stream_id);
                }
                path_vecs.push(path_vec);
            }
        }
        
        let elements = CommandBuilder::unsubscribe_many_paths(&path_vecs);
        let root = GlowRoot::with_elements(elements);
        info!("unsubscribe_many: {} paths in 1 packet", paths.len());
        self.send_only(root).await
    }

    /// Unsubscribe from changes at a path.
    /// Uses send_only (fire-and-forget) like subscribe, since unsubscribe
    /// responses are not guaranteed and send_and_receive could timeout.
    pub async fn unsubscribe(&self, path: &str) -> Result<()> {
        let path_vec = parse_path(path)?;
        
        // Remove stream mapping
        if let Some(&stream_id) = path_vec.last() {
            self.stream_to_path.write().await.remove(&stream_id);
            debug!("Removed stream mapping for: {}", stream_id);
        }
        
        let elements = CommandBuilder::unsubscribe_at_path(&path_vec);
        let root = GlowRoot::with_elements(elements);
        
        self.send_only(root).await
    }

    /// Send a keep-alive request to maintain the connection.
    pub async fn send_keepalive(&self) -> Result<()> {
        let keepalive_frame = S101Encoder::encode_keepalive_request();
        
        let mut write_guard = self.write_half.lock().await;
        if let Some(writer) = write_guard.as_mut() {
            writer.write_all(&keepalive_frame).await.map_err(Error::Io)?;
            writer.flush().await.map_err(Error::Io)?;
            debug!("Sent keep-alive request");
        }
        Ok(())
    }

    /// Process any pending data (including keep-alive requests from server).
    /// Note: With the background reader, this is now a no-op. Kept for API compatibility.
    pub async fn process_pending(&self) -> Result<()> {
        // Background reader handles all incoming data
        Ok(())
    }

    /// Invoke a function.
    pub async fn invoke(&self, path: &str, arguments: Vec<EmberValue>) -> Result<InvocationResult> {
        let path_vec = parse_path(path)?;
        
        let invocation_id = {
            let mut counter = self.request_counter.lock().await;
            *counter += 1;
            *counter
        };

        let elements = CommandBuilder::invoke_at_path(&path_vec, invocation_id, arguments);
        let root = GlowRoot::with_elements(elements);
        
        let response = self.send_and_receive(root).await?;

        // Find the invocation result
        for result in response.invocation_results {
            if result.invocation_id == invocation_id {
                return Ok(result);
            }
        }

        Err(Error::Timeout)
    }

    /// Get the local tree cache.
    pub async fn tree(&self) -> tokio::sync::RwLockReadGuard<'_, EmberTree> {
        self.tree.read().await
    }

    /// Register a callback for value updates.
    pub async fn on_value_change(&self, callback: ValueCallback) {
        self.value_callbacks.write().await.push(callback);
    }

    /// Register a callback for stream updates.
    pub async fn on_stream_update(&self, callback: StreamCallback) {
        self.stream_callbacks.write().await.push(callback);
    }

    /// Send a request without waiting for response (fire-and-forget).
    /// Used for subscriptions where responses come asynchronously via callbacks.
    pub async fn send_only(&self, request: GlowRoot) -> Result<()> {
        // Encode to Glow/BER
        let glow_bytes = codec::encode(&request)?;
        
        // Frame with S101
        let frame_bytes = S101Encoder::encode_ember_packet(&glow_bytes);

        // Send via write_half
        {
            let mut write_guard = self.write_half.lock().await;
            let writer = write_guard.as_mut().ok_or_else(|| Error::connection("Not connected"))?;
            
            writer.write_all(&frame_bytes).await.map_err(Error::Io)?;
            writer.flush().await.map_err(Error::Io)?;
        }

        debug!("Sent {} bytes (fire-and-forget)", frame_bytes.len());
        Ok(())
    }

    /// Send a request and wait for response.
    /// Send a request and receive the response.
    pub async fn send_and_receive(&self, request: GlowRoot) -> Result<GlowRoot> {
        // Encode to Glow/BER
        let glow_bytes = codec::encode(&request)?;
        
        // Frame with S101
        let frame_bytes = S101Encoder::encode_ember_packet(&glow_bytes);

        // Send via write_half
        {
            let mut write_guard = self.write_half.lock().await;
            let writer = write_guard.as_mut().ok_or_else(|| Error::connection("Not connected"))?;
            
            writer.write_all(&frame_bytes).await.map_err(Error::Io)?;
            writer.flush().await.map_err(Error::Io)?;
        }

        debug!("Sent {} bytes", frame_bytes.len());

        // Receive response
        let response = timeout(self.config.request_timeout, self.receive_response()).await
            .map_err(|_| Error::Timeout)??;

        Ok(response)
    }

    /// Receive a response from the provider via the background reader channel.
    async fn receive_response(&self) -> Result<GlowRoot> {
        let mut accumulated = GlowRoot::new();
        let mut received_any = false;

        loop {
            let wait_duration = if received_any {
                // After first message: short wait for multi-part responses.
                // Matrix connections arrive in a second packet within ~3ms on LAN.
                // Node/param lists typically fit in one packet, but use 2ms to be safe.
                Duration::from_millis(2)
            } else {
                Duration::from_secs(5)
            };
            
            let msg = {
                let mut rx_guard = self.rx.lock().await;
                let rx = rx_guard.as_mut().ok_or_else(|| Error::connection("Not connected"))?;
                
                match timeout(wait_duration, rx.recv()).await {
                    Ok(Some(msg)) => msg,
                    Ok(None) => return Err(Error::connection("Channel closed")),
                    Err(_) => {
                        if received_any {
                            return Ok(accumulated);
                        }
                        if !*self.connected.read().await {
                            return Err(Error::connection("Disconnected"));
                        }
                        continue;
                    }
                }
            };
            
            match msg {
                BackgroundMessage::GlowData(root) => {
                    accumulated.elements.extend(root.elements);
                    accumulated.invocation_results.extend(root.invocation_results);
                    accumulated.streams.extend(root.streams);
                    received_any = true;
                    // Loop back: wait_duration=2ms will time out if no more packets,
                    // catching both single-packet (nodes) and multi-packet (matrix conns).
                }
                BackgroundMessage::Disconnected => {
                    return Err(Error::connection("Disconnected"));
                }
            }
        }
    }

    /// Notify callbacks of updates.
    #[allow(dead_code)]
    async fn notify_updates(&self, root: &GlowRoot) {
        // Notify stream callbacks
        let stream_callbacks = self.stream_callbacks.read().await;
        for entry in &root.streams {
            for callback in stream_callbacks.iter() {
                callback(entry.stream_identifier, &entry.value);
            }
        }

        // Notify value callbacks for parameter updates
        let value_callbacks = self.value_callbacks.read().await;
        for element in &root.elements {
            self.notify_element_updates(element, vec![], &value_callbacks).await;
        }
    }

    /// Recursively notify callbacks for element updates.
    #[allow(dead_code)]
    async fn notify_element_updates(
        &self,
        element: &GlowElement,
        parent_path: Vec<i32>,
        callbacks: &[ValueCallback],
    ) {
        match element {
            GlowElement::Parameter(param) => {
                if let Some(value) = &param.value {
                    let mut path = parent_path;
                    path.push(param.number);
                    for callback in callbacks {
                        callback(&path, value);
                    }
                }
            }
            GlowElement::QualifiedParameter(path, param) => {
                if let Some(value) = &param.value {
                    for callback in callbacks {
                        callback(path, value);
                    }
                }
            }
            GlowElement::Node(node) => {
                let mut path = parent_path;
                path.push(node.number);
                for child in &node.children {
                    Box::pin(self.notify_element_updates(child, path.clone(), callbacks)).await;
                }
            }
            GlowElement::QualifiedNode(path, node) => {
                for child in &node.children {
                    Box::pin(self.notify_element_updates(child, path.clone(), callbacks)).await;
                }
            }
            _ => {}
        }
    }
}

impl Drop for EmberClient {
    fn drop(&mut self) {
        // Connection cleanup happens automatically when stream is dropped
    }
}
