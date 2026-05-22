//! Online battle module — WebSocket client
//! Handles communication with the game server, forwarding messages as Tauri events

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

// ── WebSocket Message Structs ─────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WsMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// ── Connection State ─────────────────────────────

pub struct OnlineState {
    /// Channel sender for sending messages
    pub sender: Option<mpsc::UnboundedSender<String>>,
    /// Our piece color
    pub my_color: Option<String>,
    /// Whether currently connected
    pub connected: bool,
}

impl OnlineState {
    pub fn new() -> Self {
        Self {
            sender: None,
            my_color: None,
            connected: false,
        }
    }
}

// ── Helper Functions ─────────────────────────────

fn make_msg<S: Into<String>>(msg_type: S) -> WsMessage {
    WsMessage {
        msg_type: msg_type.into(),
        pos: None,
        color: None,
        message: None,
    }
}

fn serialize_msg(msg: &WsMessage) -> Result<String, String> {
    serde_json::to_string(msg).map_err(|e| format!("Serialization failed: {e}"))
}

// ── Config File Reading ──────────────────────────

/// Server config structure
#[derive(Deserialize)]
struct ServerConfig {
    server_host: String,
    server_port: u16,
}

/// Read server address from compile-time embedded config.json, cross-platform compatible
fn read_server_url() -> String {
    let config: ServerConfig =
        serde_json::from_str(include_str!("../config.json")).expect("Failed to parse embedded config.json");
    format!("ws://{}:{}/ws", config.server_host, config.server_port)
}

// ── Tauri Commands ───────────────────────────────

/// Connect to the game server (reads address from config file), then maintain WebSocket connection in background
#[tauri::command]
pub async fn connect_server(
    app: AppHandle,
    state: tauri::State<'_, std::sync::Mutex<OnlineState>>,
) -> Result<(), String> {
    let ws_url = read_server_url();

    let (ws_stream, _) = connect_async(&ws_url)
        .await
        .map_err(|e| format!("Failed to connect to server: {e}"))?;

    let (mut write, mut read) = ws_stream.split();

    // channel: Tauri command → sender → background task → WebSocket
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Background task: write messages received from channel to WebSocket
    tauri::async_runtime::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if write.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Background task: read messages from WebSocket, push to frontend via Tauri events
    tauri::async_runtime::spawn(async move {
        while let Some(result) = read.next().await {
            match result {
                Ok(Message::Text(text)) => {
                    let _ = app.emit("match_event", text.to_string());
                }
                Ok(Message::Close(_)) => {
                    let _ = app.emit("match_event", r#"{"type":"disconnected"}"#);
                    break;
                }
                Err(e) => {
                    let _ = app.emit(
                        "match_event",
                        format!(r#"{{"type":"error","message":"{}"}}"#, e),
                    );
                    break;
                }
                _ => {}
            }
        }
    });

    // Save sender
    {
        let mut online = state.lock().map_err(|e| format!("State lock failed: {e}"))?;
        online.sender = Some(tx);
        online.connected = true;
        online.my_color = None;
    }

    Ok(())
}

/// Disconnect from the server
#[tauri::command]
pub fn disconnect_server(
    state: tauri::State<'_, std::sync::Mutex<OnlineState>>,
) -> Result<(), String> {
    let mut online = state.lock().map_err(|e| format!("State lock failed: {e}"))?;
    online.sender = None; // drop sender → channel closes → write task exits
    online.connected = false;
    online.my_color = None;
    Ok(())
}

/// Start matching for an opponent
#[tauri::command]
pub fn find_match(state: tauri::State<'_, std::sync::Mutex<OnlineState>>) -> Result<(), String> {
    let online = state.lock().map_err(|e| format!("State lock failed: {e}"))?;
    let sender = online.sender.as_ref().ok_or("Not connected to server")?;

    let msg = serialize_msg(&make_msg("find_match"))?;
    sender.send(msg).map_err(|e| format!("Send failed: {e}"))?;
    Ok(())
}

/// Send a move to the server
#[tauri::command]
pub fn online_send_move(
    state: tauri::State<'_, std::sync::Mutex<OnlineState>>,
    pos_index: u32,
) -> Result<(), String> {
    let online = state.lock().map_err(|e| format!("State lock failed: {e}"))?;
    let sender = online.sender.as_ref().ok_or("Not connected to server")?;

    let msg = serialize_msg(&WsMessage {
        msg_type: "move".into(),
        pos: Some(pos_index),
        color: None,
        message: None,
    })?;
    sender.send(msg).map_err(|e| format!("Send failed: {e}"))?;
    Ok(())
}

/// Send a resign message
#[tauri::command]
pub fn online_give_up(
    state: tauri::State<'_, std::sync::Mutex<OnlineState>>,
) -> Result<(), String> {
    let online = state.lock().map_err(|e| format!("State lock failed: {e}"))?;
    let sender = online.sender.as_ref().ok_or("Not connected to server")?;

    let msg = serialize_msg(&make_msg("give_up"))?;
    sender.send(msg).map_err(|e| format!("Send failed: {e}"))?;
    Ok(())
}
