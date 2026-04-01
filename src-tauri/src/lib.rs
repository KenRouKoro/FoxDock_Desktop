mod system_settings;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serialport::SerialPortType;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::mpsc as std_mpsc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;
use tauri::{Emitter, Manager, State};
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex};
use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW,
    SetupDiGetDeviceInstanceIdW, SetupDiGetDeviceRegistryPropertyW, DIGCF_ALLCLASSES,
    DIGCF_PRESENT, HDEVINFO, SP_DEVINFO_DATA, SPDRP_HARDWAREID, SPDRP_LOCATION_INFORMATION,
    SPDRP_LOCATION_PATHS,
};
use windows_sys::Win32::Foundation::{GetLastError, ERROR_INSUFFICIENT_BUFFER, ERROR_NO_MORE_ITEMS};
use windows_sys::Win32::Storage::FileSystem::{
    GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
};

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
/// Slime Smol USB CDC (matches `windows-driver/*.inf` and project naming).
const SLIME_SMOL_VID: u16 = 0x1209;
const SLIME_SMOL_TRACKER_PID: u16 = 0x7692;
const SLIME_SMOL_RECEIVER_PID: u16 = 0x7690;
const FOXDOCK_BAUD_RATE: u32 = 115_200;
const DRIVE_TYPE_NO_ROOT_DIR: u32 = 1;
const DRIVE_TYPE_REMOVABLE: u32 = 2;
const DRIVE_TYPE_FIXED: u32 = 3;
const FLASHABLE_SLOT_IDS: std::ops::RangeInclusive<u8> = 1..=5;
const SLOT_PATHS: [(u8, &str, &[u8]); 5] = [
    (1, "Hub1-P4", &[4]),
    (2, "Hub2-P1", &[3, 1]),
    (3, "Hub2-P2", &[3, 2]),
    (4, "Hub2-P3", &[3, 3]),
    (5, "Hub2-P4", &[3, 4]),
];

struct CommandRequest {
    payload: Value,
    response_tx: oneshot::Sender<Result<Value, String>>,
}

struct DockConnectionRuntime {
    port_name: String,
    command_tx: mpsc::Sender<CommandRequest>,
    shutdown_tx: std_mpsc::Sender<()>,
    thread_handle: JoinHandle<()>,
}

struct DockConnectionState {
    runtime: Mutex<Option<DockConnectionRuntime>>,
    command_guard: AsyncMutex<()>,
}

impl Default for DockConnectionState {
    fn default() -> Self {
        Self {
            runtime: Mutex::new(None),
            command_guard: AsyncMutex::new(()),
        }
    }
}

struct FirmwareJobState {
    busy: AtomicBool,
}

impl Default for FirmwareJobState {
    fn default() -> Self {
        Self {
            busy: AtomicBool::new(false),
        }
    }
}

struct FirmwareJobGuard<'a> {
    busy: &'a AtomicBool,
}

impl Drop for FirmwareJobGuard<'_> {
    fn drop(&mut self) {
        self.busy.store(false, Ordering::Release);
    }
}

fn lock_firmware_job<'a>(
    state: &'a State<'a, FirmwareJobState>,
) -> Result<FirmwareJobGuard<'a>, String> {
    state
        .busy
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| i18n_error("backend_errors.firmware_job_busy"))?;
    Ok(FirmwareJobGuard { busy: &state.busy })
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
    #[serde(default)]
    project: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    mcu: String,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

#[derive(Serialize, Deserialize, Clone)]
struct TrackerStatus {
    id: u8,
    inserted: bool,
    usb_path: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct StatusResponse {
    led: Option<bool>,
    bl_mode: Option<u8>,
    auto_sleep: Option<bool>,
    trackers: Vec<TrackerStatus>,
}

#[derive(Serialize, Deserialize)]
struct BlModeResponse {
    mode: u8,
    name: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct AutoSleepResponse {
    enabled: bool,
}

#[derive(Serialize, Deserialize)]
struct AckResponse {
    cmd: String,
    success: bool,
    msg: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppVersionInfo {
    app_name: String,
    app_version: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FirmwareProgressEvent {
    tracker_id: u8,
    phase: String,
    progress: u8,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FirmwareFlashResult {
    tracker_id: u8,
    success: bool,
    warning: bool,
    phase: String,
    progress: u8,
    message: String,
    file_name: String,
    drive_path: Option<String>,
}

const I18N_ERROR_PREFIX: &str = "i18n:";

fn i18n_error(key: &str) -> String {
    format!("{I18N_ERROR_PREFIX}{key}")
}

fn i18n_error_with_params(key: &str, params: Value) -> String {
    format!("{I18N_ERROR_PREFIX}{key}|{params}")
}

fn emit_firmware_progress(
    app_handle: &tauri::AppHandle,
    tracker_id: u8,
    phase: &str,
    progress: u8,
    message: String,
) {
    let _ = app_handle.emit(
        "firmware-progress",
        FirmwareProgressEvent {
            tracker_id,
            phase: phase.to_string(),
            progress,
            message,
        },
    );
}

fn list_matching_ports() -> Result<Vec<DockPort>, String> {
    let ports = serialport::available_ports().map_err(|e| e.to_string())?;
    let mut result = Vec::new();

    for port in ports {
        if let SerialPortType::UsbPort(usb_info) = &port.port_type {
            if usb_info.vid == FOXDOCK_VID && usb_info.pid == FOXDOCK_PID {
                let display_name = format_usb_serial_display_name(
                    &port.port_name,
                    usb_info.vid,
                    usb_info.pid,
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

/// Fixed labels for USB serial devices (Windows INF + in-app fallback).
fn usb_known_friendly_serial_label(vid: u16, pid: u16) -> Option<&'static str> {
    match (vid, pid) {
        (SLIME_SMOL_VID, SLIME_SMOL_TRACKER_PID) => Some("Slime Smol Tracker"),
        (SLIME_SMOL_VID, SLIME_SMOL_RECEIVER_PID) => Some("Slime Smol Receiver"),
        _ => None,
    }
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

fn format_usb_serial_display_name(
    port_name: &str,
    vid: u16,
    pid: u16,
    manufacturer: Option<&str>,
    product: Option<&str>,
) -> String {
    if let Some(label) = usb_known_friendly_serial_label(vid, pid) {
        return format!("{port_name} - {label}");
    }
    format_display_name(port_name, manufacturer, product)
}

fn enumerate_slime_smol_serial_ports() -> Result<Vec<DockPort>, String> {
    let ports = serialport::available_ports().map_err(|e| e.to_string())?;
    let mut result = Vec::new();

    for port in ports {
        if let SerialPortType::UsbPort(usb_info) = &port.port_type {
            if usb_known_friendly_serial_label(usb_info.vid, usb_info.pid).is_some() {
                let display_name = format_usb_serial_display_name(
                    &port.port_name,
                    usb_info.vid,
                    usb_info.pid,
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
    timeout_secs: u64,
) -> Result<Value, String> {
    let _command_guard = state.command_guard.lock().await;
    let tx = {
        let guard = state.runtime.lock().map_err(|e| e.to_string())?;
        guard
            .as_ref()
            .map(|runtime| runtime.command_tx.clone())
            .ok_or_else(|| i18n_error("backend_errors.dock_not_connected"))?
    };

    let (response_tx, response_rx) = oneshot::channel();
    tx.send(CommandRequest {
        payload,
        response_tx,
    })
    .await
    .map_err(|e| e.to_string())?;

    match tokio::time::timeout(Duration::from_secs(timeout_secs), response_rx).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err(i18n_error("backend_errors.command_interrupted")),
        Err(_) => Err(i18n_error("backend_errors.device_timeout")),
    }
}

fn spawn_serial_manager(
    app_handle: tauri::AppHandle,
    mut port: Box<dyn serialport::SerialPort>,
    mut command_rx: mpsc::Receiver<CommandRequest>,
    shutdown_rx: std_mpsc::Receiver<()>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buffer = Vec::<u8>::new();
        let mut read_chunk = [0_u8; 1024];
        let mut pending_command: Option<oneshot::Sender<Result<Value, String>>> = None;

        loop {
            if shutdown_rx.try_recv().is_ok() {
                emit_debug_log(&app_handle, "SYS", "Serial manager received shutdown signal.\n");
                if let Some(tx) = pending_command.take() {
                    let _ = tx.send(Err(i18n_error("backend_errors.command_interrupted")));
                }
                break;
            }

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
                    let _ = old_tx.send(Err(i18n_error("backend_errors.command_overridden")));
                }
                pending_command = Some(req.response_tx);
            }

            // 检查 channel 是否已关闭（断开连接时）
            if command_rx.is_closed() {
                if let Some(tx) = pending_command.take() {
                    let _ = tx.send(Err(i18n_error("backend_errors.command_interrupted")));
                }
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

        emit_debug_log(&app_handle, "SYS", "Serial manager exited and serial port dropped.\n");
    })
}

fn take_dock_runtime(
    state: &State<'_, DockConnectionState>,
) -> Result<Option<DockConnectionRuntime>, String> {
    let mut guard = state.runtime.lock().map_err(|e| e.to_string())?;
    Ok(guard.take())
}

fn close_dock_runtime(runtime: DockConnectionRuntime) -> Result<(), String> {
    let _ = runtime.shutdown_tx.send(());
    drop(runtime.command_tx);
    runtime.thread_handle.join().map_err(|_| {
        i18n_error("backend_errors.serial_manager_join_failed")
    })
}

async fn close_dock_connection(state: &State<'_, DockConnectionState>) -> Result<(), String> {
    let _command_guard = state.command_guard.lock().await;
    if let Some(runtime) = take_dock_runtime(state)? {
        tokio::task::spawn_blocking(move || close_dock_runtime(runtime))
            .await
            .map_err(|e| e.to_string())??;
    }
    Ok(())
}

fn parse_info_response(value: Value) -> Result<DockInfo, String> {
    match value.get("type").and_then(Value::as_str) {
        Some("info") => serde_json::from_value(value).map_err(|e| e.to_string()),
        _ => Err(i18n_error_with_params(
            "backend_errors.unexpected_response",
            json!({ "expected": "info", "value": value.to_string() }),
        )),
    }
}

fn parse_status_response(value: Value) -> Result<StatusResponse, String> {
    match value.get("type").and_then(Value::as_str) {
        Some("status") => serde_json::from_value(value).map_err(|e| e.to_string()),
        _ => Err(i18n_error_with_params(
            "backend_errors.unexpected_response",
            json!({ "expected": "status", "value": value.to_string() }),
        )),
    }
}

fn parse_ack_response(value: Value) -> Result<AckResponse, String> {
    match value.get("type").and_then(Value::as_str) {
        Some("ack") => serde_json::from_value(value).map_err(|e| e.to_string()),
        _ => Err(i18n_error_with_params(
            "backend_errors.unexpected_response",
            json!({ "expected": "ack", "value": value.to_string() }),
        )),
    }
}

fn parse_bl_mode_response(value: Value) -> Result<BlModeResponse, String> {
    match value.get("type").and_then(Value::as_str) {
        Some("bl_mode") => serde_json::from_value(value).map_err(|e| e.to_string()),
        _ => Err(i18n_error_with_params(
            "backend_errors.unexpected_response",
            json!({ "expected": "bl_mode", "value": value.to_string() }),
        )),
    }
}

fn parse_auto_sleep_response(value: Value) -> Result<AutoSleepResponse, String> {
    match value.get("type").and_then(Value::as_str) {
        Some("auto_sleep") => serde_json::from_value(value).map_err(|e| e.to_string()),
        _ => Err(i18n_error_with_params(
            "backend_errors.unexpected_response",
            json!({ "expected": "auto_sleep", "value": value.to_string() }),
        )),
    }
}

#[derive(Serialize, Clone)]
struct UsbNode {
    device_id: String,
    vid: Option<String>,
    pid: Option<String>,
    location_path: Option<String>,
    location_info: Option<String>,
}

fn get_usb_location_paths(app_handle: &tauri::AppHandle) -> Result<Vec<UsbNode>, String> {
    emit_debug_log(app_handle, "USB", "Starting SetupAPI device scan...\n");
    let mut nodes = Vec::new();
    let hdev: HDEVINFO = unsafe {
        SetupDiGetClassDevsW(
            std::ptr::null(),
            to_wide_null("USB").as_ptr(),
            std::ptr::null_mut(),
            DIGCF_PRESENT | DIGCF_ALLCLASSES,
        )
    };

    if hdev == -1isize {
        return Err(i18n_error("backend_errors.setupapi_failed"));
    }

    let mut index = 0;
    loop {
        let mut devinfo = SP_DEVINFO_DATA {
            cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
            ClassGuid: unsafe { std::mem::zeroed() },
            DevInst: 0,
            Reserved: 0,
        };

        let ok = unsafe { SetupDiEnumDeviceInfo(hdev, index, &mut devinfo) };
        if ok == 0 {
            let code = unsafe { GetLastError() };
            if code == ERROR_NO_MORE_ITEMS {
                break;
            }
            index += 1;
            continue;
        }
        index += 1;

        let instance_id = get_device_instance_id(hdev, &mut devinfo);
        let hardware_ids = get_registry_property_multi_sz(hdev, &mut devinfo, SPDRP_HARDWAREID);
        let source = instance_id
            .clone()
            .or_else(|| hardware_ids.as_ref().and_then(|v| v.first().cloned()));
        let Some(source_text) = source else {
            continue;
        };

        let location_path = get_registry_property_multi_sz(hdev, &mut devinfo, SPDRP_LOCATION_PATHS)
            .and_then(|v| v.first().cloned());
        let location_info = get_registry_property_sz(hdev, &mut devinfo, SPDRP_LOCATION_INFORMATION);
        let vid_pid = extract_vid_pid(&source_text);
        emit_debug_log(
            app_handle,
            "USB",
            &format!(
                "Node source: {}, VID:PID {}:{}, path: {}, info: {}\n",
                source_text,
                vid_pid.as_ref().map(|(vid, _)| vid.as_str()).unwrap_or("-"),
                vid_pid.as_ref().map(|(_, pid)| pid.as_str()).unwrap_or("-"),
                location_path.as_deref().unwrap_or("-"),
                location_info.as_deref().unwrap_or("-")
            ),
        );
        nodes.push(UsbNode {
            device_id: instance_id.unwrap_or(source_text),
            vid: vid_pid.as_ref().map(|(vid, _)| vid.clone()),
            pid: vid_pid.as_ref().map(|(_, pid)| pid.clone()),
            location_path,
            location_info,
        });
    }
    unsafe {
        SetupDiDestroyDeviceInfoList(hdev);
    }

    emit_debug_log(app_handle, "USB", &format!("Parsed {} USB-related nodes.\n", nodes.len()));
    Ok(nodes)
}

fn to_wide_null(input: &str) -> Vec<u16> {
    let mut wide: Vec<u16> = input.encode_utf16().collect();
    wide.push(0);
    wide
}

fn bytes_to_u16_vec(raw: &[u8]) -> Vec<u16> {
    raw.chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect()
}

fn parse_reg_sz(raw: &[u8]) -> Option<String> {
    let mut data = bytes_to_u16_vec(raw);
    while data.last().copied() == Some(0) {
        data.pop();
    }
    if data.is_empty() {
        return None;
    }
    Some(String::from_utf16_lossy(&data))
}

fn parse_reg_multi_sz(raw: &[u8]) -> Vec<String> {
    let data = bytes_to_u16_vec(raw);
    let mut parts = Vec::new();
    let mut start = 0;
    for (i, ch) in data.iter().enumerate() {
        if *ch == 0 {
            if i == start {
                break;
            }
            parts.push(String::from_utf16_lossy(&data[start..i]));
            start = i + 1;
        }
    }
    parts
}

fn get_registry_property_raw(
    hdev: HDEVINFO,
    devinfo: &mut SP_DEVINFO_DATA,
    property: u32,
) -> Option<Vec<u8>> {
    let mut required = 0u32;
    let mut reg_type = 0u32;
    unsafe {
        let ok = SetupDiGetDeviceRegistryPropertyW(
            hdev,
            devinfo,
            property,
            &mut reg_type,
            std::ptr::null_mut(),
            0,
            &mut required,
        );
        if ok == 0 {
            let code = GetLastError();
            if code != ERROR_INSUFFICIENT_BUFFER || required == 0 {
                return None;
            }
        }
        let mut buf = vec![0u8; required as usize];
        let ok2 = SetupDiGetDeviceRegistryPropertyW(
            hdev,
            devinfo,
            property,
            &mut reg_type,
            buf.as_mut_ptr(),
            required,
            &mut required,
        );
        if ok2 == 0 {
            return None;
        }
        Some(buf)
    }
}

fn get_registry_property_sz(
    hdev: HDEVINFO,
    devinfo: &mut SP_DEVINFO_DATA,
    property: u32,
) -> Option<String> {
    get_registry_property_raw(hdev, devinfo, property).and_then(|raw| parse_reg_sz(&raw))
}

fn get_registry_property_multi_sz(
    hdev: HDEVINFO,
    devinfo: &mut SP_DEVINFO_DATA,
    property: u32,
) -> Option<Vec<String>> {
    get_registry_property_raw(hdev, devinfo, property).map(|raw| parse_reg_multi_sz(&raw))
}

fn get_device_instance_id(hdev: HDEVINFO, devinfo: &mut SP_DEVINFO_DATA) -> Option<String> {
    let mut required = 0u32;
    unsafe {
        let ok =
            SetupDiGetDeviceInstanceIdW(hdev, devinfo, std::ptr::null_mut(), 0, &mut required);
        if ok == 0 {
            let code = GetLastError();
            if code != ERROR_INSUFFICIENT_BUFFER || required == 0 {
                return None;
            }
        }
        let mut buf = vec![0u16; required as usize + 1];
        let ok2 = SetupDiGetDeviceInstanceIdW(
            hdev,
            devinfo,
            buf.as_mut_ptr(),
            buf.len() as u32,
            &mut required,
        );
        if ok2 == 0 {
            return None;
        }
        let len = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..len]))
    }
}

fn extract_vid_pid(device_id: &str) -> Option<(String, String)> {
    // 例如: USB\VID_303A&PID_1001\6&19B86D40&0&1
    let upper = device_id.to_uppercase();
    let vid_start = upper.find("VID_")?;
    let vid = upper[vid_start + 4..vid_start + 8].to_string();
    let pid_start = upper.find("PID_")?;
    let pid = upper[pid_start + 4..pid_start + 8].to_string();
    Some((vid, pid))
}

fn remove_usbmi_suffix(path: &str) -> &str {
    if let Some(idx) = path.find("#USBMI(") {
        &path[..idx]
    } else {
        path
    }
}

fn parse_relative_usb_ports(relative_path: &str) -> Vec<u8> {
    let mut ports = Vec::new();
    let mut remain = relative_path;
    loop {
        let Some(start) = remain.find("#USB(") else {
            break;
        };
        let seg = &remain[start + 5..];
        let Some(end) = seg.find(')') else {
            break;
        };
        if let Ok(port) = seg[..end].parse::<u8>() {
            ports.push(port);
        }
        remain = &seg[end + 1..];
    }
    ports
}

fn push_tracker_unique(list: &mut Vec<TrackerStatus>, item: TrackerStatus) {
    if list.iter().any(|t| t.id == item.id) {
        return;
    }
    list.push(item);
}

fn relative_ports_match(actual: &[u8], expected: &[u8]) -> bool {
    actual.len() >= expected.len() && actual[..expected.len()] == *expected
}

fn get_slot_path(slot_id: u8) -> Option<(&'static str, &'static [u8])> {
    SLOT_PATHS
        .iter()
        .find(|(id, _, _)| *id == slot_id)
        .map(|(_, label, ports)| (*label, *ports))
}

fn get_primary_hub_path(nodes: &[UsbNode]) -> Option<String> {
    let base_node = nodes.iter().find(|node| {
        node.vid.as_deref() == Some("303A") && node.pid.as_deref() == Some("1001")
    })?;
    let base_path = base_node.location_path.as_deref().map(remove_usbmi_suffix)?;
    let last_hash = base_path.rfind('#')?;
    Some(base_path[..last_hash].to_string())
}

fn slot_present_in_nodes(nodes: &[UsbNode], primary_hub_path: &str, expected_ports: &[u8]) -> bool {
    nodes.iter().any(|node| {
        let Some(path) = node.location_path.as_deref() else {
            return false;
        };
        let path_clean = remove_usbmi_suffix(path);
        if !path_clean.starts_with(primary_hub_path) {
            return false;
        }
        let relative_path = &path_clean[primary_hub_path.len()..];
        let ports = parse_relative_usb_ports(relative_path);
        relative_ports_match(&ports, expected_ports)
    })
}

fn get_drive_label(root_path: &str) -> Option<String> {
    let wide_root = to_wide_null(root_path);
    let mut volume_name = vec![0u16; 261];
    let ok = unsafe {
        GetVolumeInformationW(
            wide_root.as_ptr(),
            volume_name.as_mut_ptr(),
            volume_name.len() as u32,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
        )
    };
    if ok == 0 {
        return None;
    }
    let len = volume_name
        .iter()
        .position(|ch| *ch == 0)
        .unwrap_or(volume_name.len());
    if len == 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&volume_name[..len]))
}

fn list_candidate_drives() -> Result<Vec<String>, String> {
    let mask = unsafe { GetLogicalDrives() };
    if mask == 0 {
        return Err(i18n_error_with_params(
            "backend_errors.logical_drives_failed",
            json!({ "code": unsafe { GetLastError() } }),
        ));
    }

    let mut drives = Vec::new();
    for index in 0..26 {
        if mask & (1 << index) == 0 {
            continue;
        }
        let letter = (b'A' + index as u8) as char;
        let root_path = format!("{letter}:\\");
        let drive_type = unsafe { GetDriveTypeW(to_wide_null(&root_path).as_ptr()) };
        if drive_type == DRIVE_TYPE_NO_ROOT_DIR {
            continue;
        }
        if drive_type == DRIVE_TYPE_REMOVABLE || drive_type == DRIVE_TYPE_FIXED {
            drives.push(root_path);
        }
    }

    drives.sort();
    Ok(drives)
}

fn sanitize_firmware_file_name(file_name: &str) -> Result<String, String> {
    Path::new(file_name)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| i18n_error("backend_errors.invalid_firmware_file_name"))
}

async fn wait_for_bootloader_drive(
    app_handle: &tauri::AppHandle,
    tracker_id: u8,
    baseline_drives: &[String],
) -> Result<String, String> {
    let baseline_set: BTreeSet<String> = baseline_drives.iter().cloned().collect();
    let (_, expected_ports) =
        get_slot_path(tracker_id).ok_or_else(|| i18n_error("backend_errors.flash_slot_out_of_range"))?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    let mut fallback_drive: Option<String> = None;

    while tokio::time::Instant::now() <= deadline {
        let current_drives = list_candidate_drives()?;
        let new_drives = current_drives
            .iter()
            .filter(|drive| !baseline_set.contains(*drive))
            .cloned()
            .collect::<Vec<_>>();

        if new_drives.len() == 1 {
            fallback_drive = new_drives.first().cloned();
        }

        let nodes = get_usb_location_paths(app_handle)?;
        if let Some(primary_hub_path) = get_primary_hub_path(&nodes) {
            let slot_ready = slot_present_in_nodes(&nodes, &primary_hub_path, expected_ports);
            if slot_ready {
                if new_drives.len() == 1 {
                    return Ok(new_drives[0].clone());
                }
                if new_drives.len() > 1 {
                    if let Some(preferred) = new_drives.iter().find_map(|drive| {
                        let label = get_drive_label(drive)?;
                        let upper = label.to_ascii_uppercase();
                        if upper.contains("UF2") || upper.contains("BOOT") {
                            Some(drive.clone())
                        } else {
                            None
                        }
                    }) {
                        return Ok(preferred);
                    }
                    return Err(i18n_error("backend_errors.bootloader_drive_ambiguous"));
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    fallback_drive.ok_or_else(|| i18n_error("backend_errors.bootloader_drive_timeout"))
}

async fn wait_for_drive_removal(drive_root: &str, timeout_secs: u64) -> Result<bool, String> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    while tokio::time::Instant::now() <= deadline {
        let drives = list_candidate_drives()?;
        if !drives.iter().any(|drive| drive.eq_ignore_ascii_case(drive_root)) {
            return Ok(true);
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Ok(false)
}

async fn copy_firmware_to_drive(
    app_handle: tauri::AppHandle,
    tracker_id: u8,
    drive_root: String,
    file_name: String,
    file_data: Vec<u8>,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let total_bytes = file_data.len();
        let target_path = Path::new(&drive_root).join(&file_name);
        let mut file = File::create(&target_path).map_err(|error| {
            i18n_error_with_params(
                "backend_errors.firmware_copy_failed",
                json!({ "msg": error.to_string() }),
            )
        })?;

        if total_bytes == 0 {
            return Err(i18n_error("backend_errors.empty_firmware_file"));
        }

        let mut offset = 0usize;
        let chunk_size = 64 * 1024;
        while offset < total_bytes {
            let end = (offset + chunk_size).min(total_bytes);
            file.write_all(&file_data[offset..end]).map_err(|error| {
                i18n_error_with_params(
                    "backend_errors.firmware_copy_failed",
                    json!({ "msg": error.to_string() }),
                )
            })?;
            offset = end;
            let progress = 20u8.saturating_add(((offset as f64 / total_bytes as f64) * 75.0) as u8);
            emit_firmware_progress(
                &app_handle,
                tracker_id,
                "copying",
                progress.min(95),
                i18n_error_with_params(
                    "flashing.progress_copying",
                    json!({ "progress": ((offset as f64 / total_bytes as f64) * 100.0).round() as u8 }),
                ),
            );
        }

        file.flush().map_err(|error| {
            i18n_error_with_params(
                "backend_errors.firmware_copy_failed",
                json!({ "msg": error.to_string() }),
            )
        })?;
        file.sync_all().map_err(|error| {
            i18n_error_with_params(
                "backend_errors.firmware_copy_failed",
                json!({ "msg": error.to_string() }),
            )
        })?;

        Ok(())
    })
    .await
    .map_err(|error| {
        i18n_error_with_params(
            "backend_errors.firmware_task_join_failed",
            json!({ "msg": error.to_string() }),
        )
    })?
}

#[tauri::command]
async fn scan_usb_topology(app_handle: tauri::AppHandle) -> Result<Vec<TrackerStatus>, String> {
    let nodes = get_usb_location_paths(&app_handle)?;
    let mut tracker_info = Vec::new();

    if let Some(base) = nodes.iter().find(|node| {
        node.vid.as_deref() == Some("303A") && node.pid.as_deref() == Some("1001")
    }) {
        emit_debug_log(
            &app_handle,
            "USB",
            &format!(
                "Found Base (303A:1001), path: {}, info: {}\n",
                base.location_path.as_deref().map(remove_usbmi_suffix).unwrap_or("-"),
                base.location_info.as_deref().unwrap_or("-")
            ),
        );

        if let Some(primary_hub_path) = get_primary_hub_path(&nodes) {
            emit_debug_log(
                &app_handle,
                "USB",
                &format!("Identified Primary HUB path: {}\n", primary_hub_path),
            );
            emit_debug_log(
                &app_handle,
                "USB",
                "Using fixed slot topology: Hub1-P4, Hub2 behind Hub1-P3 (P1..P4 => Slot2..Slot5)\n",
            );

            for (slot_id, label, expected_ports) in SLOT_PATHS {
                let found = slot_present_in_nodes(&nodes, &primary_hub_path, expected_ports);
                emit_debug_log(
                    &app_handle,
                    "USB",
                    &format!(
                        "Slot {} ({}) presence by address {:?}: {}\n",
                        slot_id,
                        label,
                        expected_ports,
                        if found { "found" } else { "not found" }
                    ),
                );
                if found {
                    push_tracker_unique(
                        &mut tracker_info,
                        TrackerStatus {
                            id: slot_id,
                            inserted: true,
                            usb_path: Some(label.to_string()),
                        },
                    );
                }
            }
        }

        if tracker_info.is_empty() {
            emit_debug_log(
                &app_handle,
                "USB",
                "No slot address matched in this scan.\n",
            );
        }
    } else {
        emit_debug_log(&app_handle, "USB", "Base device (303A:1001) not found in topology.\n");
    }
    
    emit_debug_log(&app_handle, "USB", &format!("Scan complete. Found {} trackers on this dock.\n", tracker_info.len()));
    Ok(tracker_info)
}

#[tauri::command]
async fn flash_tracker_firmware(
    app_handle: tauri::AppHandle,
    state: State<'_, DockConnectionState>,
    firmware_state: State<'_, FirmwareJobState>,
    tracker_id: u8,
    file_name: String,
    file_data: Vec<u8>,
) -> Result<FirmwareFlashResult, String> {
    let _job_guard = lock_firmware_job(&firmware_state)?;
    let safe_file_name = sanitize_firmware_file_name(&file_name)?;
    if !FLASHABLE_SLOT_IDS.contains(&tracker_id) {
        return Err(i18n_error("backend_errors.flash_slot_out_of_range"));
    }
    if !safe_file_name.to_ascii_lowercase().ends_with(".uf2") {
        return Err(i18n_error("backend_errors.invalid_firmware_file_type"));
    }
    if file_data.is_empty() {
        return Err(i18n_error("backend_errors.empty_firmware_file"));
    }

    let mut current_progress = 0u8;
    let result = async {
        let status_response = get_dock_status(state.clone()).await?;
        let inserted = status_response
            .trackers
            .iter()
            .find(|tracker| tracker.id == tracker_id)
            .map(|tracker| tracker.inserted)
            .unwrap_or(false);
        if !inserted {
            return Err(i18n_error_with_params(
                "backend_errors.flash_target_not_inserted",
                json!({ "id": tracker_id }),
            ));
        }

        let baseline_drives = list_candidate_drives()?;
        current_progress = 5;
        emit_firmware_progress(
            &app_handle,
            tracker_id,
            "entering_bl",
            current_progress,
            i18n_error_with_params("flashing.progress_entering_bl", json!({ "id": tracker_id })),
        );

        let ack = control_tracker(state.clone(), "bl".to_string(), tracker_id).await?;
        if !ack.success {
            return Err(i18n_error_with_params(
                "backend_errors.firmware_bl_command_failed",
                json!({ "msg": ack.msg.unwrap_or(ack.cmd) }),
            ));
        }

        tokio::time::sleep(Duration::from_millis(300)).await;
        current_progress = 15;
        emit_firmware_progress(
            &app_handle,
            tracker_id,
            "waiting_bootloader",
            current_progress,
            i18n_error_with_params("flashing.progress_waiting_bootloader", json!({ "id": tracker_id })),
        );

        let drive_root = wait_for_bootloader_drive(&app_handle, tracker_id, &baseline_drives).await?;
        current_progress = 20;
        emit_firmware_progress(
            &app_handle,
            tracker_id,
            "copying",
            current_progress,
            i18n_error_with_params(
                "flashing.progress_drive_ready",
                json!({ "drive": drive_root, "name": safe_file_name }),
            ),
        );

        let copy_result = copy_firmware_to_drive(
            app_handle.clone(),
            tracker_id,
            drive_root.clone(),
            safe_file_name.clone(),
            file_data,
        )
        .await;

        current_progress = 96;
        emit_firmware_progress(
            &app_handle,
            tracker_id,
            "verifying",
            current_progress,
            i18n_error_with_params("flashing.progress_waiting_removal", json!({ "drive": drive_root })),
        );

        match copy_result {
            Ok(()) => {
                if wait_for_drive_removal(&drive_root, 20).await? {
                    current_progress = 100;
                    let message = i18n_error_with_params(
                        "flashing.result_success",
                        json!({ "id": tracker_id }),
                    );
                    emit_firmware_progress(
                        &app_handle,
                        tracker_id,
                        "success",
                        current_progress,
                        message.clone(),
                    );
                    Ok(FirmwareFlashResult {
                        tracker_id,
                        success: true,
                        warning: false,
                        phase: "success".to_string(),
                        progress: current_progress,
                        message,
                        file_name: safe_file_name,
                        drive_path: Some(drive_root),
                    })
                } else {
                    Err(i18n_error_with_params(
                        "backend_errors.bootloader_drive_not_removed",
                        json!({ "drive": drive_root }),
                    ))
                }
            }
            Err(copy_error) => {
                if wait_for_drive_removal(&drive_root, 8).await? {
                    current_progress = 100;
                    let message = i18n_error_with_params(
                        "flashing.result_success_with_warning",
                        json!({ "id": tracker_id }),
                    );
                    emit_firmware_progress(
                        &app_handle,
                        tracker_id,
                        "success",
                        current_progress,
                        message.clone(),
                    );
                    Ok(FirmwareFlashResult {
                        tracker_id,
                        success: true,
                        warning: true,
                        phase: "success".to_string(),
                        progress: current_progress,
                        message,
                        file_name: safe_file_name,
                        drive_path: Some(drive_root),
                    })
                } else {
                    Err(copy_error)
                }
            }
        }
    }
    .await;

    if let Err(message) = &result {
        emit_firmware_progress(&app_handle, tracker_id, "error", current_progress, message.clone());
    }

    result
}

#[tauri::command]
async fn connect_dock(
    app_handle: tauri::AppHandle,
    state: State<'_, DockConnectionState>,
    port_name: String,
) -> Result<DockPort, String> {
    close_dock_connection(&state).await?;

    let available = list_matching_ports()?;
    let selected = available
        .iter()
        .find(|p| p.port_name == port_name)
        .cloned()
        .ok_or_else(|| i18n_error("backend_errors.port_not_found"))?;

    // 1. 打开串口
    let port = serialport::new(&port_name, FOXDOCK_BAUD_RATE)
        .timeout(Duration::from_millis(100))
        .open()
        .map_err(|e| {
            i18n_error_with_params(
                "backend_errors.open_serial_failed",
                json!({ "error": e.to_string() }),
            )
        })?;

    // 针对 ESP32C3：不要主动设置 DTR/RTS，以免触发硬件重启进入烧录模式
    // 默认保持不操作，或者如果库默认设置了，尝试不操作它。

    // 2. 创建通信通道
    let (command_tx, command_rx) = mpsc::channel(10);
    let (shutdown_tx, shutdown_rx) = std_mpsc::channel();

    // 3. 启动后台管理线程
    let thread_handle = spawn_serial_manager(app_handle.clone(), port, command_rx, shutdown_rx);

    // 4. 更新状态
    {
        let mut runtime_guard = state.runtime.lock().map_err(|e| e.to_string())?;
        *runtime_guard = Some(DockConnectionRuntime {
            port_name: port_name.clone(),
            command_tx,
            shutdown_tx,
            thread_handle,
        });
    }

    // 5. 初始状态获取（带 3 次重试）
    let mut last_err = String::new();
    for i in 0..3 {
        // 先等一下设备稳定，第一次重试等待时间更长
        let wait_ms = if i == 0 { 1000 } else { 500 };
        tokio::time::sleep(Duration::from_millis(wait_ms)).await;
        
        match send_command_via_channel(&state, json!({ "cmd": "status" }), 5).await {
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
        close_dock_connection(&state).await?;
        return Err(i18n_error_with_params(
            "backend_errors.initial_status_failed",
            json!({ "detail": last_err }),
        ));
    }

    Ok(selected)
}

#[tauri::command]
async fn disconnect_dock(state: State<'_, DockConnectionState>) -> Result<(), String> {
    close_dock_connection(&state).await
}

#[tauri::command]
fn get_connected_port(state: State<'_, DockConnectionState>) -> Result<Option<String>, String> {
    Ok(state
        .runtime
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .map(|runtime| runtime.port_name.clone()))
}

#[tauri::command]
async fn check_dock_connection(state: State<'_, DockConnectionState>) -> Result<bool, String> {
    let connected = state
        .runtime
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .map(|runtime| runtime.port_name.clone());
    let Some(port_name) = connected else {
        return Ok(false);
    };
    let alive = list_matching_ports()?.iter().any(|p| p.port_name == port_name);
    if !alive {
        close_dock_connection(&state).await?;
    }
    Ok(alive)
}

#[tauri::command]
async fn get_dock_info(
    state: State<'_, DockConnectionState>,
) -> Result<DockInfo, String> {
    let response = send_command_via_channel(&state, json!({ "cmd": "info" }), 5).await?;
    parse_info_response(response)
}

#[tauri::command]
async fn get_dock_status(
    state: State<'_, DockConnectionState>,
) -> Result<StatusResponse, String> {
    let response = send_command_via_channel(&state, json!({ "cmd": "status" }), 5).await?;
    parse_status_response(response)
}

#[tauri::command]
async fn get_bl_mode(
    state: State<'_, DockConnectionState>,
) -> Result<BlModeResponse, String> {
    let response = send_command_via_channel(&state, json!({ "cmd": "get_bl_mode" }), 5).await?;
    parse_bl_mode_response(response)
}

#[tauri::command]
async fn set_bl_mode(
    state: State<'_, DockConnectionState>,
    mode: u8,
) -> Result<AckResponse, String> {
    if !matches!(mode, 0 | 1) {
        return Err(i18n_error("backend_errors.invalid_bl_mode"));
    }
    let response = send_command_via_channel(
        &state,
        json!({
            "cmd": "set_bl_mode",
            "mode": mode
        }),
        5,
    )
    .await?;
    parse_ack_response(response)
}

#[tauri::command]
async fn get_auto_sleep(
    state: State<'_, DockConnectionState>,
) -> Result<AutoSleepResponse, String> {
    let response = send_command_via_channel(&state, json!({ "cmd": "get_auto_sleep" }), 5).await?;
    parse_auto_sleep_response(response)
}

#[tauri::command]
async fn set_auto_sleep(
    state: State<'_, DockConnectionState>,
    enabled: bool,
) -> Result<AckResponse, String> {
    let response = send_command_via_channel(
        &state,
        json!({
            "cmd": "set_auto_sleep",
            "state": if enabled { 1 } else { 0 }
        }),
        5,
    )
    .await?;
    parse_ack_response(response)
}

fn get_action_timeout(action: &str) -> u64 {
    match action {
        "ret" | "ret_all" => 4,       // 0.5s + 3s = 3.5s
        "bl" | "bl_all" => 4,        // 1s + 3s = 4s
        "sleep" | "sleep_all" => 5,   // 1.5s + 3s = 4.5s
        "wake_up" | "wake_up_all" => 4,
        "pair" | "pair_all" => 10,    // 6.5s + 3s = 9.5s
        _ => 5,
    }
}

#[tauri::command]
async fn control_tracker(
    state: State<'_, DockConnectionState>,
    action: String,
    tracker_id: u8,
) -> Result<AckResponse, String> {
    if !(1..=10).contains(&tracker_id) {
        return Err(i18n_error("backend_errors.tracker_id_out_of_range"));
    }

    if !matches!(action.as_str(), "ret" | "bl" | "sleep" | "pair" | "wake_up") {
        return Err(i18n_error("backend_errors.unsupported_single_action"));
    }

    let timeout = get_action_timeout(&action);
    let response = send_command_via_channel(
        &state,
        json!({
            "cmd": action,
            "id": tracker_id
        }),
        timeout,
    ).await?;
    parse_ack_response(response)
}

#[tauri::command]
async fn control_all(
    state: State<'_, DockConnectionState>,
    action: String,
) -> Result<AckResponse, String> {
    if !matches!(
        action.as_str(),
        "ret_all" | "bl_all" | "sleep_all" | "pair_all" | "wake_up_all"
    ) {
        return Err(i18n_error("backend_errors.unsupported_all_action"));
    }

    let timeout = get_action_timeout(&action);
    let response = send_command_via_channel(&state, json!({ "cmd": action }), timeout).await?;
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
        5,
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

#[tauri::command]
fn discover_docks() -> Result<Vec<DockPort>, String> {
    list_matching_ports()
}

/// Enumerates Slime Smol Tracker / Receiver USB serial ports (`0x1209:0x7692` / `0x1209:0x7690`) with fixed display names.
#[tauri::command]
fn list_slime_smol_serial_ports() -> Result<Vec<DockPort>, String> {
    enumerate_slime_smol_serial_ports()
}

#[tauri::command]
fn get_app_version(app_handle: tauri::AppHandle) -> AppVersionInfo {
    let package_info = app_handle.package_info();
    AppVersionInfo {
        app_name: package_info.name.clone(),
        app_version: package_info.version.to_string(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(DockConnectionState::default())
        .manage(FirmwareJobState::default())
        .invoke_handler(tauri::generate_handler![
            discover_docks,
            list_slime_smol_serial_ports,
            connect_dock,
            disconnect_dock,
            get_connected_port,
            check_dock_connection,
            get_dock_info,
            get_dock_status,
            get_bl_mode,
            set_bl_mode,
            get_auto_sleep,
            set_auto_sleep,
            control_tracker,
            control_all,
            set_dock_led,
            flash_tracker_firmware,
            open_debug_window,
            scan_usb_topology,
            get_app_version,
            system_settings::load_system_settings,
            system_settings::save_system_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
