use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serialport::SerialPortType;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{Emitter, Manager, State};
use tokio::sync::{mpsc, oneshot};

#[derive(Serialize, Clone)]
struct DebugLog {
    direction: String, // "TX" or "RX"
    content: String,
    timestamp: String,
}

fn emit_debug_log(app_handle: &tauri::AppHandle, direction: &str, content: &str) {
    let log = DebugLog {
        direction: direction.to_string(),
        content: content.to_string(),
        timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
    };
    let _ = app_handle.emit("serial-debug-log", log);
}

const FOXDOCK_VID: u16 = 0x303A;
const FOXDOCK_PID: u16 = 0x1001;
const FOXDOCK_BAUD_RATE: u32 = 115_200;

struct CommandRequest {
    payload: Value,
    response_tx: oneshot::Sender<Result<Value, String>>,
}

struct DockConnectionState {
    command_tx: Mutex<Option<mpsc::Sender<CommandRequest>>>,
    port_name: Mutex<Option<String>>,
}

impl Default for DockConnectionState {
    fn default() -> Self {
        Self {
            command_tx: Mutex::new(None),
            port_name: Mutex::new(None),
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DockPort {
    port_name: String,
    display_name: String,
    serial_number: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct DockInfo {
    project: String,
    version: String,
    mcu: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct TrackerStatus {
    id: u8,
    inserted: bool,
}

#[derive(Serialize, Deserialize)]
struct StatusResponse {
    trackers: Vec<TrackerStatus>,
}

#[derive(Serialize, Deserialize)]
struct AckResponse {
    cmd: String,
    success: bool,
    msg: Option<String>,
}

fn list_matching_ports() -> Result<Vec<DockPort>, String> {
    let ports = serialport::available_ports().map_err(|e| e.to_string())?;
    let mut result = Vec::new();

    for port in ports {
        if let SerialPortType::UsbPort(usb_info) = &port.port_type {
            if usb_info.vid == FOXDOCK_VID && usb_info.pid == FOXDOCK_PID {
                let display_name = format_display_name(
                    &port.port_name,
                    usb_info.manufacturer.as_deref(),
                    usb_info.product.as_deref(),
                );

                result.push(DockPort {
                    port_name: port.port_name.clone(),
                    display_name,
                    serial_number: usb_info.serial_number.clone(),
                });
            }
        }
    }

    Ok(result)
}

fn format_display_name(
    port_name: &str,
    manufacturer: Option<&str>,
    product: Option<&str>,
) -> String {
    match (manufacturer, product) {
        (Some(m), Some(p)) => format!("{port_name} - {m} {p}"),
        (Some(m), None) => format!("{port_name} - {m}"),
        (None, Some(p)) => format!("{port_name} - {p}"),
        (None, None) => port_name.to_string(),
    }
}

fn extract_first_json(buffer: &mut Vec<u8>) -> Option<Value> {
    let text = String::from_utf8_lossy(buffer);
    
    // 寻找第一个 '{'
    if let Some(start_idx) = text.find('{') {
        // 从这个 '{' 开始往后找所有的 '}'
        let potential_part = &text[start_idx..];
        
        // 尝试寻找每一个 '}'，看哪一个能构成合法的 JSON
        for (rel_end_idx, c) in potential_part.char_indices() {
            if c == '}' {
                let end_idx = start_idx + rel_end_idx;
                let candidate = &text[start_idx..=end_idx];
                
                if let Ok(value) = serde_json::from_str::<Value>(candidate) {
                    // 找到了一个合法的 JSON 对象
                    let bytes_to_remove = text[..=end_idx].as_bytes().len();
                    buffer.drain(..bytes_to_remove);
                    return Some(value);
                }
            }
        }
        
        // 如果 buffer 已经很大了（比如超过 10KB）且还没有找到闭合的 JSON，
        // 说明前面可能有脏数据，清理掉第一个 '{' 之前的所有内容。
        if buffer.len() > 10240 {
             let bytes_to_drain = text[..=start_idx].as_bytes().len();
             buffer.drain(..bytes_to_drain);
        }
    } else if buffer.len() > 1024 {
        // 如果没有 '{' 且 buffer 积累较多，清理 buffer
        buffer.clear();
    }
    
    None
}

async fn send_command_via_channel(
    state: &State<'_, DockConnectionState>,
    payload: Value,
) -> Result<Value, String> {
    let tx = {
        let guard = state.command_tx.lock().map_err(|e| e.to_string())?;
        guard.as_ref().cloned().ok_or("当前未连接底座")?
    };

    let (response_tx, response_rx) = oneshot::channel();
    tx.send(CommandRequest {
        payload,
        response_tx,
    })
    .await
    .map_err(|e| e.to_string())?;

    match tokio::time::timeout(Duration::from_secs(5), response_rx).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err("命令执行中途中断".to_string()),
        Err(_) => Err("设备响应超时".to_string()),
    }
}

fn spawn_serial_manager(
    app_handle: tauri::AppHandle,
    mut port: Box<dyn serialport::SerialPort>,
    mut command_rx: mpsc::Receiver<CommandRequest>,
) {
    std::thread::spawn(move || {
        let mut buffer = Vec::<u8>::new();
        let mut read_chunk = [0_u8; 1024];
        let mut pending_command: Option<oneshot::Sender<Result<Value, String>>> = None;

        loop {
            // 1. 处理来自前端的命令
            // 改为阻塞读取，直到收到新命令、或者 channel 被关闭、或者需要读取串口
            // 使用 recv() 会阻塞，所以我们使用 select 风格或者保持循环但优化结构
            while let Ok(req) = command_rx.try_recv() {
                let command_line = format!("{}\n", req.payload);
                emit_debug_log(&app_handle, "TX", &command_line);
                
                if let Err(e) = port.write_all(command_line.as_bytes()) {
                    let _ = req.response_tx.send(Err(e.to_string()));
                    continue;
                }
                let _ = port.flush();
                
                if let Some(old_tx) = pending_command.take() {
                    let _ = old_tx.send(Err("被新命令覆盖".to_string()));
                }
                pending_command = Some(req.response_tx);
            }

            // 检查 channel 是否已关闭（断开连接时）
            if command_rx.is_closed() && pending_command.is_none() {
                break;
            }

            // 2. 读取串口数据
            match port.read(&mut read_chunk) {
                Ok(read_len) if read_len > 0 => {
                    let chunk = &read_chunk[..read_len];
                    emit_debug_log(&app_handle, "RX", &String::from_utf8_lossy(chunk));
                    buffer.extend_from_slice(chunk);

                    // 循环解析 buffer 中的所有 JSON
                    while let Some(value) = extract_first_json(&mut buffer) {
                        let msg_type = value.get("type").and_then(Value::as_str);
                        
                        if msg_type == Some("event") {
                            // 主动上报事件
                            let _ = app_handle.emit("dock-event", &value);
                        } else if pending_command.is_some() {
                            // 响应包 (ack, status, info)
                            if let Some(tx) = pending_command.take() {
                                let _ = tx.send(Ok(value));
                            }
                        } else {
                            // 无人认领的响应包，作为事件发给前端
                            let _ = app_handle.emit("dock-event", &value);
                        }
                    }
                }
                Ok(_) => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(_) => {
                    break;
                }
            }
        }
    });
}

fn parse_info_response(value: Value) -> Result<DockInfo, String> {
    match value.get("type").and_then(Value::as_str) {
        Some("info") => serde_json::from_value(value).map_err(|e| e.to_string()),
        _ => Err(format!("设备返回的不是 info 响应。内容: {value}")),
    }
}

fn parse_status_response(value: Value) -> Result<Vec<TrackerStatus>, String> {
    match value.get("type").and_then(Value::as_str) {
        Some("status") => {
            let parsed: StatusResponse = serde_json::from_value(value).map_err(|e| e.to_string())?;
            Ok(parsed.trackers)
        }
        _ => Err(format!("设备返回的不是 status 响应。内容: {value}")),
    }
}

fn parse_ack_response(value: Value) -> Result<AckResponse, String> {
    match value.get("type").and_then(Value::as_str) {
        Some("ack") => serde_json::from_value(value).map_err(|e| e.to_string()),
        _ => Err(format!("设备返回的不是 ack 响应。内容: {value}")),
    }
}

#[tauri::command]
fn discover_docks() -> Result<Vec<DockPort>, String> {
    list_matching_ports()
}

#[tauri::command]
async fn connect_dock(
    app_handle: tauri::AppHandle,
    state: State<'_, DockConnectionState>,
    port_name: String,
) -> Result<DockPort, String> {
    let available = list_matching_ports()?;
    let selected = available
        .iter()
        .find(|p| p.port_name == port_name)
        .cloned()
        .ok_or_else(|| "未找到指定底座端口，请先刷新设备列表".to_string())?;

    // 1. 打开串口
    let port = serialport::new(&port_name, FOXDOCK_BAUD_RATE)
        .timeout(Duration::from_millis(100))
        .open()
        .map_err(|e| format!("无法打开串口: {e}"))?;

    // 针对 ESP32C3：不要主动设置 DTR/RTS，以免触发硬件重启进入烧录模式
    // 默认保持不操作，或者如果库默认设置了，尝试不操作它。

    // 2. 创建通信通道
    let (command_tx, command_rx) = mpsc::channel(10);

    // 3. 启动后台管理线程
    spawn_serial_manager(app_handle.clone(), port, command_rx);

    // 4. 更新状态
    {
        let mut tx_guard = state.command_tx.lock().map_err(|e| e.to_string())?;
        let mut name_guard = state.port_name.lock().map_err(|e| e.to_string())?;
        *tx_guard = Some(command_tx);
        *name_guard = Some(port_name);
    }

    // 5. 初始状态获取（带 3 次重试）
    let mut last_err = String::new();
    for i in 0..3 {
        // 先等一下设备稳定，第一次重试等待时间更长
        let wait_ms = if i == 0 { 1000 } else { 500 };
        tokio::time::sleep(Duration::from_millis(wait_ms)).await;
        
        match send_command_via_channel(&state, json!({ "cmd": "status" })).await {
            Ok(_) => {
                last_err.clear();
                break;
            },
            Err(e) => {
                last_err = e;
            }
        }
    }
    
    if !last_err.is_empty() {
        return Err(format!("连接成功但无法获取初始状态: {}", last_err));
    }

    Ok(selected)
}

#[tauri::command]
async fn disconnect_dock(state: State<'_, DockConnectionState>) -> Result<(), String> {
    let mut tx_guard = state.command_tx.lock().map_err(|e| e.to_string())?;
    let mut name_guard = state.port_name.lock().map_err(|e| e.to_string())?;
    *tx_guard = None; // 这会导致后台线程退出
    *name_guard = None;
    Ok(())
}

#[tauri::command]
fn get_connected_port(state: State<'_, DockConnectionState>) -> Result<Option<String>, String> {
    Ok(state.port_name.lock().map_err(|e| e.to_string())?.clone())
}

#[tauri::command]
async fn get_dock_info(
    state: State<'_, DockConnectionState>,
) -> Result<DockInfo, String> {
    let response = send_command_via_channel(&state, json!({ "cmd": "info" })).await?;
    parse_info_response(response)
}

#[tauri::command]
async fn get_dock_status(
    state: State<'_, DockConnectionState>,
) -> Result<Vec<TrackerStatus>, String> {
    let response = send_command_via_channel(&state, json!({ "cmd": "status" })).await?;
    parse_status_response(response)
}

#[tauri::command]
async fn control_tracker(
    state: State<'_, DockConnectionState>,
    action: String,
    tracker_id: u8,
) -> Result<AckResponse, String> {
    if !(1..=10).contains(&tracker_id) {
        return Err("追踪器 ID 必须在 1 到 10 之间".to_string());
    }

    if !matches!(action.as_str(), "ret" | "bl" | "sleep" | "pair") {
        return Err("不支持的单点控制动作".to_string());
    }

    let response = send_command_via_channel(
        &state,
        json!({
            "cmd": action,
            "id": tracker_id
        }),
    ).await?;
    parse_ack_response(response)
}

#[tauri::command]
async fn control_all(
    state: State<'_, DockConnectionState>,
    action: String,
) -> Result<AckResponse, String> {
    if !matches!(action.as_str(), "ret_all" | "bl_all" | "sleep_all" | "pair_all") {
        return Err("不支持的全体控制动作".to_string());
    }

    let response = send_command_via_channel(&state, json!({ "cmd": action })).await?;
    parse_ack_response(response)
}

#[tauri::command]
async fn set_dock_led(
    state: State<'_, DockConnectionState>,
    enabled: bool,
) -> Result<AckResponse, String> {
    let response = send_command_via_channel(
        &state,
        json!({
            "cmd": "led",
            "state": if enabled { 1 } else { 0 }
        }),
    ).await?;
    parse_ack_response(response)
}

#[tauri::command]
async fn open_debug_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    // 使用 query parameter 而非 path，以适配 Vite dev server
    let debug_window = tauri::WebviewWindowBuilder::new(
        &app_handle,
        "debug",
        tauri::WebviewUrl::App("/?debug=true".into()),
    )
    .title("Serial Debug Console")
    .inner_size(600.0, 400.0)
    .resizable(true)
    .build();

    match debug_window {
        Ok(_) => Ok(()),
        Err(e) => {
            // 如果窗口已存在，则尝试获取并聚焦
            if let Some(window) = app_handle.get_webview_window("debug") {
                let _ = window.set_focus();
                Ok(())
            } else {
                Err(e.to_string())
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(DockConnectionState::default())
        .invoke_handler(tauri::generate_handler![
            discover_docks,
            connect_dock,
            disconnect_dock,
            get_connected_port,
            get_dock_info,
            get_dock_status,
            control_tracker,
            control_all,
            set_dock_led,
            open_debug_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
