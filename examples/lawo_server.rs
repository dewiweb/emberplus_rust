//! WebSocket server for Lawo MC²36 control.
//!
//! Serves a WebSocket API for the frontend to control and monitor the console.
//! Run with: cargo run --example lawo_server

#![allow(dead_code)]

use ember_plus::{EmberClient, EmberValue, Result as EmberResult};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, error, info};

const LAWO_ADDRESS: &str = "192.168.100.199:9000";
const WS_PORT: u16 = 9001;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum WsMessage {
    // Client -> Server
    #[serde(rename = "connect")]
    Connect { address: String },
    #[serde(rename = "disconnect")]
    Disconnect,
    #[serde(rename = "loadCategory")]
    LoadCategory { category: String },
    #[serde(rename = "setFader")]
    SetFader { id: i32, value: f64 },
    #[serde(rename = "setMute")]
    SetMute { id: i32, mute: bool },
    #[serde(rename = "setPfl")]
    SetPfl { id: i32, pfl: bool },
    #[serde(rename = "refresh")]
    Refresh,

    // Server -> Client
    #[serde(rename = "connected")]
    Connected { categories: Vec<CategoryInfo> },
    #[serde(rename = "disconnected")]
    Disconnected,
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "categoryLoaded")]
    CategoryLoaded { category: String, channels: Vec<ChannelInfo> },
    #[serde(rename = "faderState")]
    FaderState(FaderData),
    #[serde(rename = "meterUpdate")]
    MeterUpdate { id: i32, level: f64, peak: f64 },
    #[serde(rename = "discoveryProgress")]
    DiscoveryProgress { current: i32, total: i32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CategoryInfo {
    key: String,
    name: String,
    path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChannelInfo {
    number: i32,
    name: String,
    path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct FaderData {
    id: i32,
    label: String,
    #[serde(rename = "faderValue")]
    fader_value: f64,
    mute: bool,
    pfl: bool,
    #[serde(rename = "meterLevel")]
    meter_level: f64,
}

struct AppState {
    client: Option<EmberClient>,
    faders: HashMap<i32, FaderData>,
    channel_paths: HashMap<i32, ChannelPaths>,
    /// Reverse map: path -> (fader_id, param_type)
    path_to_fader: HashMap<String, (i32, ParamType)>,
    /// Categories discovered from console
    categories: HashMap<String, CategoryInfo>,
    /// Sapphire root path
    sapphire_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ParamType {
    FaderLevel,
    Mute,
    Pfl,
}

#[derive(Debug, Clone, Default)]
struct ChannelPaths {
    base_path: String,
    name: Option<String>,
    fader_path: Option<String>,
    mute_path: Option<String>,
    pfl_path: Option<String>,
    meter_path: Option<String>,
}

impl AppState {
    fn new() -> Self {
        let mut faders = HashMap::new();
        for id in 1..=24 {
            faders.insert(id, FaderData {
                id,
                label: format!("CH {}", id),
                fader_value: f64::NEG_INFINITY,
                ..Default::default()
            });
        }
        Self {
            client: None,
            faders,
            channel_paths: HashMap::new(),
            path_to_fader: HashMap::new(),
            categories: HashMap::new(),
            sapphire_path: String::new(),
        }
    }
}

type SharedState = Arc<RwLock<AppState>>;
type BroadcastTx = broadcast::Sender<WsMessage>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("lawo_server=debug,ember_plus=info")
        .init();

    let state = Arc::new(RwLock::new(AppState::new()));
    let (broadcast_tx, _) = broadcast::channel::<WsMessage>(100);

    let addr = format!("0.0.0.0:{}", WS_PORT);
    let listener = TcpListener::bind(&addr).await?;
    
    info!("╔════════════════════════════════════════════════════════════╗");
    info!("║     LAWO MC²36 WebSocket Server                            ║");
    info!("╠════════════════════════════════════════════════════════════╣");
    info!("║  WebSocket: ws://localhost:{}                             ║", WS_PORT);
    info!("║  Console:   {}                              ║", LAWO_ADDRESS);
    info!("╚════════════════════════════════════════════════════════════╝");

    while let Ok((stream, addr)) = listener.accept().await {
        let state = state.clone();
        let broadcast_tx = broadcast_tx.clone();
        tokio::spawn(handle_connection(stream, addr, state, broadcast_tx));
    }

    Ok(())
}

async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    state: SharedState,
    broadcast_tx: BroadcastTx,
) {
    info!("New connection from {}", addr);

    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("WebSocket handshake failed: {}", e);
            return;
        }
    };

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut broadcast_rx = broadcast_tx.subscribe();

    // Send initial fader states
    {
        let state_guard = state.read().await;
        for fader in state_guard.faders.values() {
            let msg = WsMessage::FaderState(fader.clone());
            if let Ok(json) = serde_json::to_string(&msg) {
                let _ = ws_sender.send(Message::Text(json)).await;
            }
        }
    }

    loop {
        tokio::select! {
            // Handle incoming messages from client
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                            handle_client_message(ws_msg, &state, &broadcast_tx, &mut ws_sender).await;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        info!("Client {} disconnected", addr);
                        break;
                    }
                    _ => {}
                }
            }
            // Forward broadcast messages to client
            msg = broadcast_rx.recv() => {
                if let Ok(ws_msg) = msg {
                    if let Ok(json) = serde_json::to_string(&ws_msg) {
                        if ws_sender.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    }
}

async fn handle_client_message(
    msg: WsMessage,
    state: &SharedState,
    broadcast_tx: &BroadcastTx,
    _ws_sender: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<TcpStream>,
        Message,
    >,
) {
    match msg {
        WsMessage::Connect { address } => {
            info!("Connecting to console at {}", address);
            
            match connect_to_console(&address, state, broadcast_tx).await {
                Ok(categories) => {
                    let _ = broadcast_tx.send(WsMessage::Connected { categories });
                }
                Err(e) => {
                    let _ = broadcast_tx.send(WsMessage::Error { message: e.to_string() });
                }
            }
        }
        WsMessage::LoadCategory { category } => {
            info!("Loading category: {}", category);
            match load_category(&category, state, broadcast_tx).await {
                Ok(_) => {}
                Err(e) => {
                    let _ = broadcast_tx.send(WsMessage::Error { message: e.to_string() });
                }
            }
        }
        WsMessage::Disconnect => {
            info!("Disconnecting from console");
            let mut state_guard = state.write().await;
            if let Some(client) = state_guard.client.take() {
                let _ = client.disconnect().await;
            }
            let _ = broadcast_tx.send(WsMessage::Disconnected);
        }
        WsMessage::SetFader { id, value } => {
            debug!("Set fader {} to {}", id, value);
            let state_guard = state.read().await;
            if let Some(ref client) = state_guard.client {
                if let Some(channel) = state_guard.channel_paths.get(&id) {
                    if let Some(ref path) = channel.fader_path {
                        let _ = client.set_value(path, EmberValue::Real(value)).await;
                    }
                }
            }
        }
        WsMessage::SetMute { id, mute } => {
            debug!("Set fader {} mute to {}", id, mute);
            let state_guard = state.read().await;
            if let Some(ref client) = state_guard.client {
                if let Some(channel) = state_guard.channel_paths.get(&id) {
                    if let Some(ref path) = channel.mute_path {
                        let _ = client.set_value(path, EmberValue::Boolean(mute)).await;
                    }
                }
            }
        }
        WsMessage::SetPfl { id, pfl } => {
            debug!("Set fader {} PFL to {}", id, pfl);
            let state_guard = state.read().await;
            if let Some(ref client) = state_guard.client {
                if let Some(channel) = state_guard.channel_paths.get(&id) {
                    if let Some(ref path) = channel.pfl_path {
                        let _ = client.set_value(path, EmberValue::Boolean(pfl)).await;
                    }
                }
            }
        }
        WsMessage::Refresh => {
            let state_guard = state.read().await;
            for fader in state_guard.faders.values() {
                let _ = broadcast_tx.send(WsMessage::FaderState(fader.clone()));
            }
        }
        _ => {}
    }
}

/// Find a Real value parameter inside a node (explores 2 levels deep)
async fn find_real_param(client: &EmberClient, parent_path: &str) -> Option<(String, f64)> {
    // Fetch parent to ensure children are loaded
    if client.get_element_by_path(parent_path).await.is_err() {
        return None;
    }

    // Get children numbers
    let child_nums: Vec<i32> = {
        let tree = client.tree().await;
        if let Ok(Some(node)) = tree.get_by_path_str(parent_path) {
            let guard = node.read();
            guard.children().map(|c| c.read().number()).collect()
        } else {
            return None;
        }
    };

    // Check each child for a Real value
    for child_num in child_nums {
        let child_path = format!("{}.{}", parent_path, child_num);
        if let Ok(Some(node)) = client.get_element_by_path(&child_path).await {
            let guard = node.read();
            // Check if this node has a Real value
            if let Some(EmberValue::Real(v)) = guard.value() {
                return Some((child_path, *v));
            }
            // Also check for Integer that could represent dB (scaled)
            if let Some(EmberValue::Integer(v)) = guard.value() {
                // Lawo uses factor=32 for fader, so divide by 32
                let db = (*v as f64) / 32.0;
                if db <= 15.0 && db >= -100.0 {
                    return Some((child_path, db));
                }
            }
        }
    }
    None
}

/// Find a Boolean value parameter inside a node
async fn find_bool_param(client: &EmberClient, parent_path: &str) -> Option<(String, bool)> {
    let child_nums: Vec<i32> = {
        let tree = client.tree().await;
        if let Ok(Some(node)) = tree.get_by_path_str(parent_path) {
            let guard = node.read();
            guard.children().map(|c| c.read().number()).collect()
        } else {
            return None;
        }
    };

    for child_num in child_nums {
        let child_path = format!("{}.{}", parent_path, child_num);
        if let Ok(Some(node)) = client.get_element_by_path(&child_path).await {
            let guard = node.read();
            if let Some(val) = guard.value() {
                match val {
                    EmberValue::Boolean(v) => return Some((child_path, *v)),
                    EmberValue::Integer(v) => return Some((child_path, *v != 0)),
                    _ => {}
                }
            }
        }
    }
    None
}

/// Find a String value parameter inside a node (for User Label)
async fn find_string_param(client: &EmberClient, parent_path: &str, target_id: &str) -> Option<String> {
    // First expand the parent to get children
    let _ = client.get_element_by_path(parent_path).await;
    
    let child_nums: Vec<i32> = {
        let tree = client.tree().await;
        if let Ok(Some(node)) = tree.get_by_path_str(parent_path) {
            let guard = node.read();
            guard.children().map(|c| c.read().number()).collect()
        } else {
            return None;
        }
    };

    for child_num in child_nums {
        let child_path = format!("{}.{}", parent_path, child_num);
        if let Ok(Some(node)) = client.get_element_by_path(&child_path).await {
            let guard = node.read();
            let id = guard.identifier().unwrap_or("");
            
            // Match by identifier (_60010004 = User Label)
            if id == target_id {
                if let Some(EmberValue::String(s)) = guard.value() {
                    return Some(s.clone());
                }
            }
        }
    }
    None
}

/// Find a child node by identifier and return its path
async fn find_child_by_id(client: &EmberClient, parent_path: &str, target_id: &str) -> Option<String> {
    // First expand the parent to ensure children are loaded with their identifiers
    let _ = client.expand(parent_path).await;
    
    // Get children info from local tree
    let children: Vec<(i32, String)> = {
        let tree = client.tree().await;
        if let Ok(Some(node)) = tree.get_by_path_str(parent_path) {
            let guard = node.read();
            guard.children()
                .map(|c| {
                    let cg = c.read();
                    (cg.number(), cg.identifier().unwrap_or("").to_string())
                })
                .collect()
        } else {
            return None;
        }
    };

    // Find child with matching identifier
    for (num, id) in &children {
        if id == target_id {
            return Some(format!("{}.{}", parent_path, num));
        }
    }
    
    // If not found by identifier, try expanding each child to get identifiers
    for (num, _) in children.iter().take(15) {
        let child_path = format!("{}.{}", parent_path, num);
        let _ = client.expand(&child_path).await;
        
        let id: String = {
            let tree = client.tree().await;
            if let Ok(Some(node)) = tree.get_by_path_str(&child_path) {
                let guard = node.read();
                guard.identifier().unwrap_or("").to_string()
            } else {
                continue;
            }
        };
        
        if id == target_id {
            return Some(child_path);
        }
    }
    
    None
}

async fn connect_to_console(
    address: &str,
    state: &SharedState,
    broadcast_tx: &BroadcastTx,
) -> EmberResult<Vec<CategoryInfo>> {
    info!("Connecting to Ember+ at {}", address);
    let client = EmberClient::connect(address).await?;
    
    // 1. Set up value change callback FIRST - will receive values when we subscribe
    let callback_state = state.clone();
    let callback_tx = broadcast_tx.clone();
    client.on_value_change(Box::new(move |path, value| {
        let path_str = path.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(".");
        let state = callback_state.clone();
        let tx = callback_tx.clone();
        let value = value.clone();
        
        tokio::spawn(async move {
            let mapping = {
                let sg = state.read().await;
                sg.path_to_fader.get(&path_str).cloned()
            };
            
            if let Some((fader_id, param_type)) = mapping {
                let mut sg = state.write().await;
                if let Some(fader) = sg.faders.get_mut(&fader_id) {
                    match param_type {
                        ParamType::FaderLevel => {
                            let db = match &value {
                                EmberValue::Real(v) => *v,
                                EmberValue::Integer(v) => (*v as f64) / 32.0,
                                _ => return,
                            };
                            fader.fader_value = db;
                            info!("Fader {} = {} dB", fader_id, db);
                        }
                        ParamType::Mute => {
                            let mute = match &value {
                                EmberValue::Boolean(v) => *v,
                                EmberValue::Integer(v) => *v != 0,
                                _ => return,
                            };
                            fader.mute = mute;
                            info!("Mute {} = {}", fader_id, mute);
                        }
                        _ => {}
                    }
                    let _ = tx.send(WsMessage::FaderState(fader.clone()));
                }
            }
        });
    })).await;

    // 2. Get root directory
    let _ = client.get_directory().await?;

    // 3. FAST DISCOVERY: Get categories structure (~5ms)
    let root_children = client.expand_with_info("").await.unwrap_or_default();
    
    // Find Sapphire node (_2)
    let sapphire = root_children.iter()
        .find(|c| c.identifier.as_deref() == Some("_2"));
    let sapphire_num = sapphire.map(|s| s.number).unwrap_or(28548);
    let sapphire_path = format!("{}", sapphire_num);
    
    // Get Sapphire children (categories)
    let sapphire_children = client.expand_with_info(&sapphire_path).await.unwrap_or_default();
    
    // Known categories
    let category_keys = [
        ("Sums", "Sums"),
        ("Auxes", "Auxes"),
        ("Masters", "Masters"),
        ("Groups", "Groups"),
        ("Monitors", "Monitors"),
        ("Listen Sums", "ListenSums"),
    ];
    
    // Build category paths
    let mut cat_paths: Vec<(String, String, String)> = Vec::new();
    for child in &sapphire_children {
        let desc = child.description.as_deref().unwrap_or("");
        for (cat_desc, key) in &category_keys {
            if desc == *cat_desc {
                let cat_path = format!("{}.{}", sapphire_path, child.number);
                cat_paths.push((key.to_string(), desc.to_string(), cat_path));
                break;
            }
        }
    }
    
    info!("Found {} categories, loading channels in PARALLEL...", cat_paths.len());
    
    // 4. PARALLEL LOADING: Each category uses its own connection (~15ms total)
    use futures_util::future::join_all;
    let address_clone = address.to_string();
    
    let futures: Vec<_> = cat_paths.iter()
        .map(|(key, name, path)| {
            let key = key.clone();
            let name = name.clone();
            let path = path.clone();
            let addr = address_clone.clone();
            async move {
                // Each category gets its own connection for true parallelism
                let cat_client = match EmberClient::connect(&addr).await {
                    Ok(c) => c,
                    Err(_) => return None,
                };
                let _ = cat_client.get_directory().await;
                let channels = cat_client.expand_with_info(&path).await.unwrap_or_default();
                let _ = cat_client.disconnect().await;
                
                let channel_infos: Vec<ChannelInfo> = channels.iter()
                    .map(|c| ChannelInfo {
                        number: c.number,
                        name: c.description.as_deref().unwrap_or("?").to_string(),
                        path: format!("{}.{}", path, c.number),
                    })
                    .collect();
                
                Some((key, name, path, channel_infos))
            }
        })
        .collect();
    
    let results = join_all(futures).await;
    
    // 5. Store results
    let mut categories = Vec::new();
    let mut total_channels = 0;
    
    for result in results.into_iter().flatten() {
        let (key, name, path, channels) = result;
        total_channels += channels.len();
        
        let cat_info = CategoryInfo {
            key: key.clone(),
            name: name.clone(),
            path: path.clone(),
        };
        categories.push(cat_info.clone());
        
        state.write().await.categories.insert(key.clone(), cat_info);
        
        // Send channels to frontend
        let _ = broadcast_tx.send(WsMessage::CategoryLoaded {
            category: key,
            channels,
        });
    }
    
    // Store main client
    {
        let mut sg = state.write().await;
        sg.sapphire_path = sapphire_path;
        sg.client = Some(client);
    }
    
    info!("Discovery complete: {} categories, {} channels total", categories.len(), total_channels);
    
    Ok(categories)
}

/// Load channels for a specific category (lazy loading)
async fn load_category(
    category_key: &str,
    state: &SharedState,
    broadcast_tx: &BroadcastTx,
) -> EmberResult<()> {
    let (client_exists, cat_path) = {
        let sg = state.read().await;
        let exists = sg.client.is_some();
        let path = sg.categories.get(category_key).map(|c| c.path.clone());
        (exists, path)
    };
    
    if !client_exists {
        return Err(ember_plus::Error::Connection("Not connected".to_string()));
    }
    
    let cat_path = match cat_path {
        Some(p) => p,
        None => return Err(ember_plus::Error::PathNotFound(category_key.to_string())),
    };
    
    info!("Loading category {} at {}", category_key, cat_path);
    
    // Get channels using expand_with_info
    let channels_info = {
        let sg = state.read().await;
        if let Some(ref client) = sg.client {
            client.expand_with_info(&cat_path).await.unwrap_or_default()
        } else {
            vec![]
        }
    };
    
    let channels: Vec<ChannelInfo> = channels_info.iter()
        .map(|c| ChannelInfo {
            number: c.number,
            name: c.description.as_deref().unwrap_or("?").to_string(),
            path: format!("{}.{}", cat_path, c.number),
        })
        .collect();
    
    info!("Loaded {} channels for {}", channels.len(), category_key);
    
    let _ = broadcast_tx.send(WsMessage::CategoryLoaded {
        category: category_key.to_string(),
        channels,
    });
    
    Ok(())
}

/// Get User Label from General node
async fn get_user_label(client: &EmberClient, general_path: &str) -> Option<String> {
    let tree = client.tree().await;
    if let Ok(Some(node)) = tree.get_by_path_str(general_path) {
        for child in node.read().children() {
            let g = child.read();
            if g.identifier() == Some("_60010004") {
                if let Some(EmberValue::String(s)) = g.value() {
                    if !s.is_empty() {
                        return Some(s.clone());
                    }
                }
            }
        }
    }
    None
}

/// Get first child parameter path
async fn get_first_param(client: &EmberClient, parent_path: &str) -> Option<String> {
    let tree = client.tree().await;
    if let Ok(Some(node)) = tree.get_by_path_str(parent_path) {
        for child in node.read().children() {
            let g = child.read();
            if g.value().is_some() {
                return Some(format!("{}.{}", parent_path, g.number()));
            }
        }
    }
    None
}
