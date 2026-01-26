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
    GlowRoot, GlowElement, GlowNode, GlowParameter, EmberPath,
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

/// Callback type for stream updates.
pub type StreamCallback = Box<dyn Fn(i32, &EmberValue) + Send + Sync>;

/// An Ember+ client for connecting to providers.
pub struct EmberClient {
    /// Remote address
    address: String,
    /// Configuration
    config: ClientConfig,
    /// TCP stream
    stream: Arc<Mutex<Option<TcpStream>>>,
    /// S101 codec
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

        let client = EmberClient {
            address: address.to_string(),
            config,
            stream: Arc::new(Mutex::new(Some(stream))),
            codec: Arc::new(Mutex::new(S101Codec::new())),
            tree: Arc::new(RwLock::new(EmberTree::new())),
            connected: Arc::new(RwLock::new(true)),
            request_counter: Arc::new(Mutex::new(0)),
            value_callbacks: Arc::new(RwLock::new(Vec::new())),
            stream_callbacks: Arc::new(RwLock::new(Vec::new())),
        };

        Ok(client)
    }

    /// Check if connected.
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Disconnect from the provider.
    pub async fn disconnect(&self) -> Result<()> {
        info!("Disconnecting from {}", self.address);
        
        *self.connected.write().await = false;
        
        if let Some(stream) = self.stream.lock().await.take() {
            drop(stream);
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

    /// Expand a specific path.
    async fn expand_path(&self, path: &[i32]) -> Result<()> {
        let path_vec = path.to_vec();
        let elements = CommandBuilder::get_directory_at_path(&path_vec);
        let root = GlowRoot::with_elements(elements);
        let response = self.send_and_receive(root).await?;

        self.tree.write().await.update_from_glow(&response);

        // Recursively expand children
        for element in &response.elements {
            self.expand_element(element, path.to_vec()).await?;
        }

        Ok(())
    }

    /// Recursively expand an element's children.
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
    pub async fn subscribe(&self, path: &str) -> Result<()> {
        let path_vec = parse_path(path)?;
        let elements = CommandBuilder::subscribe_at_path(&path_vec);
        let root = GlowRoot::with_elements(elements);
        
        self.send_and_receive(root).await?;
        Ok(())
    }

    /// Unsubscribe from changes at a path.
    pub async fn unsubscribe(&self, path: &str) -> Result<()> {
        let path_vec = parse_path(path)?;
        let elements = CommandBuilder::unsubscribe_at_path(&path_vec);
        let root = GlowRoot::with_elements(elements);
        
        self.send_and_receive(root).await?;
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

    /// Send a request and wait for response.
    async fn send_and_receive(&self, request: GlowRoot) -> Result<GlowRoot> {
        // Encode to Glow/BER
        let glow_bytes = codec::encode(&request)?;
        
        // Frame with S101
        let frame_bytes = S101Encoder::encode_ember_packet(&glow_bytes);

        // Send
        {
            let mut stream_guard = self.stream.lock().await;
            let stream = stream_guard.as_mut().ok_or_else(|| Error::connection("Not connected"))?;
            
            stream.write_all(&frame_bytes).await.map_err(Error::Io)?;
            stream.flush().await.map_err(Error::Io)?;
        }

        debug!("Sent {} bytes", frame_bytes.len());

        // Receive response
        let response = timeout(self.config.request_timeout, self.receive_response()).await
            .map_err(|_| Error::Timeout)??;

        Ok(response)
    }

    /// Receive a response from the provider.
    async fn receive_response(&self) -> Result<GlowRoot> {
        let mut buffer = vec![0u8; 65536];
        let mut accumulated = GlowRoot::new();

        loop {
            let n = {
                let mut stream_guard = self.stream.lock().await;
                let stream = stream_guard.as_mut().ok_or_else(|| Error::connection("Not connected"))?;
                stream.read(&mut buffer).await.map_err(Error::Io)?
            };

            if n == 0 {
                return Err(Error::connection("Connection closed"));
            }

            debug!("Received {} bytes", n);

            // Feed to S101 decoder
            let mut codec = self.codec.lock().await;
            codec.feed(&buffer[..n]);

            // Try to decode frames
            while let Ok(Some(frame)) = codec.decode_frame() {
                let message = frame.to_message();

                match message {
                    S101Message::EmberData { payload, flags, .. } => {
                        // Decode Glow payload
                        if let Ok(root) = codec::decode(&payload) {
                            // Update tree first (before moving data)
                            self.tree.write().await.update_from_glow(&root);

                            // Notify callbacks
                            self.notify_updates(&root).await;

                            // Merge into accumulated response
                            accumulated.elements.extend(root.elements);
                            accumulated.invocation_results.extend(root.invocation_results);
                            accumulated.streams.extend(root.streams);

                            if flags.last_packet {
                                return Ok(accumulated);
                            }
                        }
                    }
                    S101Message::KeepAliveRequest => {
                        // Send keep-alive response
                        let response = S101Encoder::encode_keepalive_response();
                        let mut stream_guard = self.stream.lock().await;
                        if let Some(stream) = stream_guard.as_mut() {
                            let _ = stream.write_all(&response).await;
                        }
                    }
                    S101Message::KeepAliveResponse => {
                        debug!("Received keep-alive response");
                    }
                }
            }

            // If we got data and it seems complete, return
            if !accumulated.elements.is_empty() || !accumulated.invocation_results.is_empty() {
                return Ok(accumulated);
            }
        }
    }

    /// Notify callbacks of updates.
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
