//! 联网对战模块 — WebSocket 客户端
//! 负责与 game server 通信，将消息转发为 Tauri 事件

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

// ── WebSocket 消息结构 ────────────────────────────

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

// ── 连接状态 ─────────────────────────────────────

pub struct OnlineState {
    /// 发送消息用的 channel sender
    pub sender: Option<mpsc::UnboundedSender<String>>,
    /// 我方执子颜色
    pub my_color: Option<String>,
    /// 是否已连接
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

// ── 辅助函数 ─────────────────────────────────────

fn make_msg<S: Into<String>>(msg_type: S) -> WsMessage {
    WsMessage {
        msg_type: msg_type.into(),
        pos: None,
        color: None,
        message: None,
    }
}

fn serialize_msg(msg: &WsMessage) -> Result<String, String> {
    serde_json::to_string(msg).map_err(|e| format!("序列化失败: {e}"))
}

// ── 配置文件读取 ────────────────────────────────

/// 服务器配置结构
#[derive(Deserialize)]
struct ServerConfig {
    server_host: String,
    server_port: u16,
}

/// 从编译期嵌入的 config.json 读取服务器地址，跨平台兼容
fn read_server_url() -> String {
    let config: ServerConfig =
        serde_json::from_str(include_str!("../config.json")).expect("无法解析嵌入的 config.json");
    format!("ws://{}:{}/ws", config.server_host, config.server_port)
}

// ── Tauri 命令 ────────────────────────────────────

/// 连接到游戏服务器（从配置文件读取服务器地址），成功后在后台维持 WebSocket 连接
#[tauri::command]
pub async fn connect_server(
    app: AppHandle,
    state: tauri::State<'_, std::sync::Mutex<OnlineState>>,
) -> Result<(), String> {
    let ws_url = read_server_url();

    let (ws_stream, _) = connect_async(&ws_url)
        .await
        .map_err(|e| format!("连接服务器失败: {e}"))?;

    let (mut write, mut read) = ws_stream.split();

    // channel: Tauri 命令 → sender → 后台任务 → WebSocket
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // 后台任务：把 channel 收到的消息写入 WebSocket
    tauri::async_runtime::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if write.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // 后台任务：从 WebSocket 读取消息，通过 Tauri 事件推给前端
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

    // 保存 sender
    {
        let mut online = state.lock().map_err(|e| format!("状态锁失败: {e}"))?;
        online.sender = Some(tx);
        online.connected = true;
        online.my_color = None;
    }

    Ok(())
}

/// 断开与服务器的连接
#[tauri::command]
pub fn disconnect_server(
    state: tauri::State<'_, std::sync::Mutex<OnlineState>>,
) -> Result<(), String> {
    let mut online = state.lock().map_err(|e| format!("状态锁失败: {e}"))?;
    online.sender = None; // 丢弃 sender → channel 关闭 → 写入任务退出
    online.connected = false;
    online.my_color = None;
    Ok(())
}

/// 开始匹配对手
#[tauri::command]
pub fn find_match(state: tauri::State<'_, std::sync::Mutex<OnlineState>>) -> Result<(), String> {
    let online = state.lock().map_err(|e| format!("状态锁失败: {e}"))?;
    let sender = online.sender.as_ref().ok_or("未连接到服务器")?;

    let msg = serialize_msg(&make_msg("find_match"))?;
    sender.send(msg).map_err(|e| format!("发送失败: {e}"))?;
    Ok(())
}

/// 发送落子到服务器
#[tauri::command]
pub fn online_send_move(
    state: tauri::State<'_, std::sync::Mutex<OnlineState>>,
    pos_index: u32,
) -> Result<(), String> {
    let online = state.lock().map_err(|e| format!("状态锁失败: {e}"))?;
    let sender = online.sender.as_ref().ok_or("未连接到服务器")?;

    let msg = serialize_msg(&WsMessage {
        msg_type: "move".into(),
        pos: Some(pos_index),
        color: None,
        message: None,
    })?;
    sender.send(msg).map_err(|e| format!("发送失败: {e}"))?;
    Ok(())
}

/// 发送认输消息
#[tauri::command]
pub fn online_give_up(
    state: tauri::State<'_, std::sync::Mutex<OnlineState>>,
) -> Result<(), String> {
    let online = state.lock().map_err(|e| format!("状态锁失败: {e}"))?;
    let sender = online.sender.as_ref().ok_or("未连接到服务器")?;

    let msg = serialize_msg(&make_msg("give_up"))?;
    sender.send(msg).map_err(|e| format!("发送失败: {e}"))?;
    Ok(())
}
