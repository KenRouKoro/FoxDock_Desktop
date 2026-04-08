//! Nordic Legacy BLE DFU (Adafruit nRF52 Bootloader) host for Windows via btleplug (WinRT BLE).

use btleplug::api::{
    BDAddr, Central, Manager as _, Peripheral as _, ScanFilter,
};
use btleplug::platform::PeripheralId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;
use windows::{
    core::{GUID, Ref},
    Devices::Bluetooth::{
        BluetoothCacheMode, BluetoothLEDevice, BluetoothLEPreferredConnectionParameters,
        GenericAttributeProfile::{
            GattCharacteristic, GattClientCharacteristicConfigurationDescriptorValue,
            GattCommunicationStatus, GattDeviceService, GattSession, GattValueChangedEventArgs,
            GattWriteOption,
        },
    },
    Foundation::TypedEventHandler,
    Storage::Streams::{DataReader, DataWriter},
};

const I18N_ERROR_PREFIX: &str = "i18n:";

fn i18n_error(key: &str) -> String {
    format!("{I18N_ERROR_PREFIX}{key}")
}

fn i18n_error_with_params(key: &str, params: serde_json::Value) -> String {
    format!("{I18N_ERROR_PREFIX}{key}|{params}")
}

// Nordic legacy base: 0000xxxx-1212-efde-1523-785feabcd123
fn nordic_dfu_uuid(short: u16) -> Uuid {
    let s = format!("{:04x}", short);
    Uuid::parse_str(&format!("0000{}-1212-efde-1523-785feabcd123", s)).expect("uuid")
}

const DFU_DEVICE_NAMES: [&str; 2] = ["AdaDFU", "FoxSnackLiteV2DFU"];

const OP_START: u8 = 0x01;
const OP_INIT: u8 = 0x02;
const OP_RECEIVE_FW: u8 = 0x03;
const OP_VALIDATE: u8 = 0x04;
const OP_ACTIVATE: u8 = 0x05;
const OP_PRN: u8 = 0x08;
#[allow(dead_code)]
const OP_PRN_NOTIFY: u8 = 0x11;
const OP_RESPONSE: u8 = 0x10;
const RESP_SUCCESS: u8 = 0x01;

const UPLOAD_MODE_APPLICATION: u8 = 0x04;

/// Packet Receipt Notification 间隔 N：每 N 个固件包设备发一次 0x11 通知。
/// WinRT 直连后链路吞吐明显改善，先把 PRN 从 4 调到 6，减少等待通知次数以继续提速。
const DFU_PRN_INTERVAL: u16 = 6;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BleOtaProgressEvent {
    pub phase: String,
    pub progress: u8,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_transferred: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_bytes: Option<u64>,
}

fn emit_ble_ota_progress(
    app: &AppHandle,
    phase: &str,
    progress: u8,
    message: String,
    bytes_transferred: Option<u64>,
    total_bytes: Option<u64>,
) {
    let _ = app.emit(
        "ble-ota-progress",
        BleOtaProgressEvent {
            phase: phase.to_string(),
            progress,
            message,
            bytes_transferred,
            total_bytes,
        },
    );
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BluetoothAdapterInfo {
    pub id: String,
    pub name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BluetoothAvailableResult {
    pub available: bool,
    pub adapters: Vec<BluetoothAdapterInfo>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BleDfuDevice {
    pub peripheral_id: String,
    pub name: String,
    pub address: String,
    pub rssi: Option<i16>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtaPackageValidationResult {
    pub firmware_type: String,
    pub bin_file_name: String,
    pub dat_file_name: String,
    pub bin_size: u64,
    pub dat_size: u64,
}

/// 共享同一 `btleplug::Adapter` 实例。Windows 上已扫描到的 `Peripheral` 缓存在该 Adapter 内；
/// 若每次命令都 `Manager::new()`，会得到空缓存，`peripheral(id)` 永远失败。
pub struct BleAdapterState {
    pub inner: tokio::sync::Mutex<Option<btleplug::platform::Adapter>>,
}

impl Default for BleAdapterState {
    fn default() -> Self {
        Self {
            inner: tokio::sync::Mutex::new(None),
        }
    }
}

async fn get_shared_adapter(
    adapter_state: &State<'_, BleAdapterState>,
) -> Result<btleplug::platform::Adapter, String> {
    let guard = adapter_state.inner.lock().await;
    if let Some(ref a) = *guard {
        return Ok(a.clone());
    }
    drop(guard);

    let manager = btleplug::platform::Manager::new()
        .await
        .map_err(|e| {
            i18n_error_with_params(
                "backend_errors.ble_ota_manager_failed",
                serde_json::json!({ "error": e.to_string() }),
            )
        })?;
    let adapters = manager.adapters().await.map_err(|e| e.to_string())?;
    let central = adapters
        .into_iter()
        .next()
        .ok_or_else(|| i18n_error("backend_errors.ble_ota_no_adapter"))?;

    let mut guard = adapter_state.inner.lock().await;
    *guard = Some(central.clone());
    eprintln!("[ble_ota] get_shared_adapter: initialized and cached first adapter");
    Ok(central)
}

pub struct BleOtaJobState {
    busy: AtomicBool,
    cancel: Arc<AtomicBool>,
}

impl Default for BleOtaJobState {
    fn default() -> Self {
        Self {
            busy: AtomicBool::new(false),
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}

struct BleOtaJobGuard<'a> {
    busy: &'a AtomicBool,
}

impl Drop for BleOtaJobGuard<'_> {
    fn drop(&mut self) {
        self.busy.store(false, Ordering::Release);
    }
}

fn lock_ble_ota_job<'a>(
    state: &'a State<'a, BleOtaJobState>,
) -> Result<BleOtaJobGuard<'a>, String> {
    state
        .busy
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| i18n_error("backend_errors.ble_ota_job_busy"))?;
    Ok(BleOtaJobGuard { busy: &state.busy })
}

/// Parse adafruit-nrfutil ZIP: manifest.json + application.bin/dat.
fn parse_ota_zip(zip_bytes: &[u8]) -> Result<(Vec<u8>, Vec<u8>, OtaPackageValidationResult), String> {
    let reader = Cursor::new(zip_bytes);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| i18n_error_with_params(
            "backend_errors.ble_ota_invalid_zip",
            serde_json::json!({ "error": e.to_string() }),
        ))?;

    let mut manifest_str = String::new();
    {
        let mut f = archive.by_name("manifest.json").map_err(|_| {
            i18n_error("backend_errors.ble_ota_manifest_missing")
        })?;
        std::io::Read::read_to_string(&mut f, &mut manifest_str).map_err(|e| e.to_string())?;
    }

    let manifest: Value = serde_json::from_str(&manifest_str)
        .map_err(|_| i18n_error("backend_errors.ble_ota_manifest_invalid_json"))?;

    let app_entry = manifest
        .get("manifest")
        .and_then(|m| m.get("application"))
        .ok_or_else(|| i18n_error("backend_errors.ble_ota_manifest_no_application"))?;

    let bin_file = app_entry
        .get("bin_file")
        .and_then(|v| v.as_str())
        .ok_or_else(|| i18n_error("backend_errors.ble_ota_manifest_bin_missing"))?;
    let dat_file = app_entry
        .get("dat_file")
        .and_then(|v| v.as_str())
        .ok_or_else(|| i18n_error("backend_errors.ble_ota_manifest_dat_missing"))?;

    let bin_data = read_zip_file(&mut archive, bin_file)?;
    let dat_data = read_zip_file(&mut archive, dat_file)?;

    if bin_data.is_empty() {
        return Err(i18n_error("backend_errors.ble_ota_bin_empty"));
    }
    if bin_data.len() % 4 != 0 {
        return Err(i18n_error_with_params(
            "backend_errors.ble_ota_bin_not_word_aligned",
            serde_json::json!({ "size": bin_data.len() }),
        ));
    }
    if dat_data.is_empty() {
        return Err(i18n_error("backend_errors.ble_ota_dat_empty"));
    }

    let bin_size = bin_data.len() as u64;
    let dat_size = dat_data.len() as u64;

    Ok((
        bin_data,
        dat_data,
        OtaPackageValidationResult {
            firmware_type: "application".to_string(),
            bin_file_name: bin_file.to_string(),
            dat_file_name: dat_file.to_string(),
            bin_size,
            dat_size,
        },
    ))
}

fn read_zip_file(archive: &mut zip::ZipArchive<Cursor<&[u8]>>, name: &str) -> Result<Vec<u8>, String> {
    let mut f = archive
        .by_name(name)
        .map_err(|_| i18n_error_with_params(
            "backend_errors.ble_ota_zip_entry_missing",
            serde_json::json!({ "name": name }),
        ))?;
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

#[tauri::command]
pub async fn check_bluetooth_available(
    adapter_state: State<'_, BleAdapterState>,
) -> Result<BluetoothAvailableResult, String> {
    let manager = btleplug::platform::Manager::new()
        .await
        .map_err(|e| {
            i18n_error_with_params(
                "backend_errors.ble_ota_manager_failed",
                serde_json::json!({ "error": e.to_string() }),
            )
        })?;
    let adapters_vec = manager.adapters().await.map_err(|e| e.to_string())?;

    let mut infos = Vec::new();
    for (idx, a) in adapters_vec.iter().enumerate() {
        let detail = a.adapter_info().await.unwrap_or_default();
        infos.push(BluetoothAdapterInfo {
            id: format!("adapter_{idx}"),
            name: detail,
        });
    }

    if let Some(first) = adapters_vec.into_iter().next() {
        *adapter_state.inner.lock().await = Some(first);
        eprintln!("[ble_ota] check_bluetooth_available: cached first adapter for subsequent scan/OTA");
    }

    Ok(BluetoothAvailableResult {
        available: !infos.is_empty(),
        adapters: infos,
    })
}

#[tauri::command]
pub async fn scan_ble_dfu_devices(
    adapter_state: State<'_, BleAdapterState>,
) -> Result<Vec<BleDfuDevice>, String> {
    let central = get_shared_adapter(&adapter_state).await?;

    central
        .start_scan(ScanFilter::default())
        .await
        .map_err(|e| e.to_string())?;
    tokio::time::sleep(Duration::from_secs(5)).await;
    central.stop_scan().await.map_err(|e| e.to_string())?;

    let peripherals = central.peripherals().await.map_err(|e| e.to_string())?;

    let mut seen: HashMap<String, BleDfuDevice> = HashMap::new();

    for p in peripherals {
        let props_opt = p.properties().await.map_err(|e| e.to_string())?;
        let Some(props) = props_opt else {
            continue;
        };
        let local = props
            .local_name
            .as_deref()
            .unwrap_or("")
            .to_string();
        if !DFU_DEVICE_NAMES.iter().any(|n| n == &local) {
            continue;
        }

        let id = p.id().to_string();
        let addr = format!("{}", props.address);

        seen.insert(
            id.clone(),
            BleDfuDevice {
                peripheral_id: id,
                name: local,
                address: addr,
                rssi: props.rssi,
            },
        );
    }

    Ok(seen.into_values().collect())
}

#[tauri::command]
pub async fn validate_ota_package(zip_bytes: Vec<u8>) -> Result<OtaPackageValidationResult, String> {
    if zip_bytes.is_empty() {
        return Err(i18n_error("backend_errors.ble_ota_zip_empty"));
    }
    let (_bin, _dat, info) = parse_ota_zip(&zip_bytes)?;
    Ok(info)
}

async fn find_peripheral_by_id(
    adapter_state: &State<'_, BleAdapterState>,
    peripheral_id: &str,
) -> Result<btleplug::platform::Peripheral, String> {
    eprintln!("[ble_ota] find_peripheral_by_id: id={}", peripheral_id);

    let addr: BDAddr = peripheral_id
        .parse()
        .map_err(|e| {
            eprintln!("[ble_ota] BDAddr parse failed: {:?} err={:?}", peripheral_id, e);
            i18n_error("backend_errors.ble_ota_invalid_peripheral_id")
        })?;
    let id = PeripheralId::from(addr);

    let central = get_shared_adapter(adapter_state).await?;

    if let Ok(p) = central.peripheral(&id).await {
        eprintln!("[ble_ota] peripheral() cache hit");
        return Ok(p);
    }

    let list = central.peripherals().await.map_err(|e| e.to_string())?;
    eprintln!(
        "[ble_ota] peripheral() miss; scanning {} cached peripherals",
        list.len()
    );
    for p in &list {
        eprintln!("[ble_ota]   id={}", p.id());
    }

    for p in list {
        if p.id().to_string() == peripheral_id {
            eprintln!("[ble_ota] matched by id string equality");
            return Ok(p);
        }
    }

    eprintln!("[ble_ota] short rescan 3s to refresh cache...");
    central
        .start_scan(ScanFilter::default())
        .await
        .map_err(|e| e.to_string())?;
    tokio::time::sleep(Duration::from_secs(3)).await;
    central.stop_scan().await.map_err(|e| e.to_string())?;

    if let Ok(p) = central.peripheral(&id).await {
        eprintln!("[ble_ota] peripheral() hit after rescan");
        return Ok(p);
    }

    let list = central.peripherals().await.map_err(|e| e.to_string())?;
    eprintln!("[ble_ota] after rescan: {} peripherals", list.len());
    for p in list {
        if p.id().to_string() == peripheral_id {
            eprintln!("[ble_ota] matched after rescan");
            return Ok(p);
        }
    }

    eprintln!("[ble_ota] find_peripheral_by_id: FAILED");
    Err(i18n_error("backend_errors.ble_ota_peripheral_not_found"))
}

#[derive(Debug)]
struct BleNotification {
    uuid: Uuid,
    value: Vec<u8>,
}

async fn wait_cp_response(
    notifications: &mut tokio::sync::mpsc::UnboundedReceiver<BleNotification>,
    cp_uuid: Uuid,
    want_procedure: u8,
    timeout: Duration,
) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(i18n_error_with_params(
                "backend_errors.ble_ota_dfu_timeout",
                serde_json::json!({ "procedure": want_procedure }),
            ));
        }
        let next = tokio::time::timeout(remaining, notifications.recv())
            .await
            .map_err(|_| {
                i18n_error_with_params(
                    "backend_errors.ble_ota_dfu_timeout",
                    serde_json::json!({ "procedure": want_procedure }),
                )
            })?;

        let Some(notification) = next else {
            return Err(i18n_error("backend_errors.ble_ota_disconnected"));
        };
        if notification.uuid != cp_uuid {
            continue;
        }
        let v = notification.value;
        if v.len() >= 3 && v[0] == OP_RESPONSE && v[1] == want_procedure {
            if v[2] == RESP_SUCCESS {
                return Ok(());
            }
            let st = v[2];
            // Nordic Legacy: 0x05/0x06 常与 CRC 或内部校验失败相关；procedure 3 = 固件流
            if want_procedure == OP_RECEIVE_FW && (st == 0x05 || st == 0x06) {
                return Err(i18n_error_with_params(
                    "backend_errors.ble_ota_dfu_fw_integrity",
                    serde_json::json!({ "status": st }),
                ));
            }
            return Err(i18n_error_with_params(
                "backend_errors.ble_ota_dfu_response_error",
                serde_json::json!({
                    "procedure": want_procedure,
                    "status": st,
                }),
            ));
        }
    }
}

enum FirmwareTransferEvent {
    Prn(u32),
    Complete,
}

async fn wait_firmware_transfer_event(
    notifications: &mut tokio::sync::mpsc::UnboundedReceiver<BleNotification>,
    cp_uuid: Uuid,
    expected_min_bytes: u32,
    timeout: Duration,
) -> Result<FirmwareTransferEvent, String> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(i18n_error_with_params(
                "backend_errors.ble_ota_prn_timeout",
                serde_json::json!({ "expected": expected_min_bytes }),
            ));
        }
        let next = tokio::time::timeout(remaining, notifications.recv())
            .await
            .map_err(|_| {
                i18n_error_with_params(
                    "backend_errors.ble_ota_prn_timeout",
                    serde_json::json!({ "expected": expected_min_bytes }),
                )
            })?;

        let Some(notification) = next else {
            return Err(i18n_error("backend_errors.ble_ota_disconnected"));
        };
        if notification.uuid != cp_uuid {
            continue;
        }

        let v = notification.value;
        if v.len() >= 5 && v[0] == OP_PRN_NOTIFY {
            let received = u32::from_le_bytes([v[1], v[2], v[3], v[4]]);
            eprintln!(
                "[ble_ota] prn notify: received_bytes={} expected_min={}",
                received, expected_min_bytes
            );
            if received >= expected_min_bytes {
                return Ok(FirmwareTransferEvent::Prn(received));
            }
            continue;
        }

        if v.len() >= 3 && v[0] == OP_RESPONSE && v[1] == OP_RECEIVE_FW {
            if v[2] == RESP_SUCCESS {
                return Ok(FirmwareTransferEvent::Complete);
            }
            let st = v[2];
            if st == 0x05 || st == 0x06 {
                return Err(i18n_error_with_params(
                    "backend_errors.ble_ota_dfu_fw_integrity",
                    serde_json::json!({ "status": st }),
                ));
            }
            return Err(i18n_error_with_params(
                "backend_errors.ble_ota_dfu_response_error",
                serde_json::json!({
                    "procedure": OP_RECEIVE_FW,
                    "status": st,
                }),
            ));
        }
    }
}

/// Max payload per Packet write (word-aligned).
/// Windows WinRT 自动与设备协商 MTU（典型 247），btleplug 0.11 不暴露协商结果，
/// 但 WriteWithoutResponse 会按底层协商后的 MTU 拆包。
/// 因此这里直接使用 244（= 247 - 3 ATT header，且 % 4 == 0）；
/// 实际底层会进一步按协商 MTU 拆（若协商低于 247，驱动自动分），不会比旧的 16 字节更差。
fn packet_chunk_len() -> usize {
    // 247 - 3 = 244, 244 / 4 * 4 = 244
    244
}

fn uuid_to_guid(uuid: Uuid) -> GUID {
    GUID::from_u128(uuid.as_u128())
}

fn parse_ble_address_u64(peripheral_id: &str) -> Result<u64, String> {
    let compact = peripheral_id.replace([':', '-'], "");
    if compact.len() != 12 || !compact.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(i18n_error_with_params(
            "backend_errors.ble_ota_invalid_peripheral_id",
            serde_json::json!({ "id": peripheral_id }),
        ));
    }
    u64::from_str_radix(&compact, 16).map_err(|_| {
        i18n_error_with_params(
            "backend_errors.ble_ota_invalid_peripheral_id",
            serde_json::json!({ "id": peripheral_id }),
        )
    })
}

fn gatt_status_error(prefix: &str, status: GattCommunicationStatus, protocol_error: Option<u8>) -> String {
    i18n_error_with_params(
        "backend_errors.ble_ota_gatt_failed",
        serde_json::json!({
            "operation": prefix,
            "status": format!("{status:?}"),
            "protocolError": protocol_error,
        }),
    )
}

fn protocol_error_value<T>(
    result: &T,
    get_ref: impl Fn(&T) -> windows::core::Result<windows::Foundation::IReference<u8>>,
) -> Option<u8> {
    get_ref(result)
        .ok()
        .and_then(|reference| reference.Value().ok())
}

fn writer_buffer(data: &[u8]) -> Result<windows::Storage::Streams::IBuffer, String> {
    let writer = DataWriter::new().map_err(|e| e.to_string())?;
    writer.WriteBytes(data).map_err(|e| e.to_string())?;
    writer.DetachBuffer().map_err(|e| e.to_string())
}

async fn winrt_write(
    characteristic: &GattCharacteristic,
    data: &[u8],
    write_option: GattWriteOption,
    operation: &str,
) -> Result<(), String> {
    let async_op = {
        let buffer = writer_buffer(data)?;
        characteristic
            .WriteValueWithResultAndOptionAsync(&buffer, write_option)
            .map_err(|e| e.to_string())?
    };
    let result = async_op
        .await
        .map_err(|e| e.to_string())?;
    let status = result.Status().map_err(|e| e.to_string())?;
    if status == GattCommunicationStatus::Success {
        return Ok(());
    }
    let protocol_error = protocol_error_value(&result, |r| r.ProtocolError());
    Err(gatt_status_error(operation, status, protocol_error))
}

async fn winrt_set_notify(characteristic: &GattCharacteristic, enable: bool) -> Result<(), String> {
    let value = if enable {
        GattClientCharacteristicConfigurationDescriptorValue::Notify
    } else {
        GattClientCharacteristicConfigurationDescriptorValue::None
    };
    let status = characteristic
        .WriteClientCharacteristicConfigurationDescriptorAsync(value)
        .map_err(|e| e.to_string())?
        .await
        .map_err(|e| e.to_string())?;
    if status == GattCommunicationStatus::Success {
        return Ok(());
    }
    Err(gatt_status_error(
        if enable { "enable_notify" } else { "disable_notify" },
        status,
        None,
    ))
}

async fn first_service_by_uuid(
    device: &BluetoothLEDevice,
    uuid: Uuid,
) -> Result<GattDeviceService, String> {
    let result = device
        .GetGattServicesForUuidWithCacheModeAsync(uuid_to_guid(uuid), BluetoothCacheMode::Uncached)
        .map_err(|e| e.to_string())?
        .await
        .map_err(|e| e.to_string())?;
    let status = result.Status().map_err(|e| e.to_string())?;
    if status != GattCommunicationStatus::Success {
        return Err(gatt_status_error(
            "get_service",
            status,
            protocol_error_value(&result, |r| r.ProtocolError()),
        ));
    }
    let services = result.Services().map_err(|e| e.to_string())?;
    if services.Size().map_err(|e| e.to_string())? == 0 {
        return Err(i18n_error("backend_errors.ble_ota_cp_missing"));
    }
    services.GetAt(0).map_err(|e| e.to_string())
}

async fn first_characteristic_by_uuid(
    service: &GattDeviceService,
    uuid: Uuid,
    missing_error_key: &str,
) -> Result<GattCharacteristic, String> {
    let result = service
        .GetCharacteristicsForUuidWithCacheModeAsync(uuid_to_guid(uuid), BluetoothCacheMode::Uncached)
        .map_err(|e| e.to_string())?
        .await
        .map_err(|e| e.to_string())?;
    let status = result.Status().map_err(|e| e.to_string())?;
    if status != GattCommunicationStatus::Success {
        return Err(gatt_status_error(
            "get_characteristic",
            status,
            protocol_error_value(&result, |r| r.ProtocolError()),
        ));
    }
    let characteristics = result.Characteristics().map_err(|e| e.to_string())?;
    if characteristics.Size().map_err(|e| e.to_string())? == 0 {
        return Err(i18n_error(missing_error_key));
    }
    characteristics.GetAt(0).map_err(|e| e.to_string())
}

async fn read_characteristic_uncached(characteristic: &GattCharacteristic) -> Result<Vec<u8>, String> {
    let result = characteristic
        .ReadValueWithCacheModeAsync(BluetoothCacheMode::Uncached)
        .map_err(|e| e.to_string())?
        .await
        .map_err(|e| e.to_string())?;
    let status = result.Status().map_err(|e| e.to_string())?;
    if status != GattCommunicationStatus::Success {
        return Err(gatt_status_error(
            "read_characteristic",
            status,
            protocol_error_value(&result, |r| r.ProtocolError()),
        ));
    }
    let value = result.Value().map_err(|e| e.to_string())?;
    let reader = DataReader::FromBuffer(&value).map_err(|e| e.to_string())?;
    let len = reader.UnconsumedBufferLength().map_err(|e| e.to_string())? as usize;
    let mut input = vec![0u8; len];
    reader.ReadBytes(&mut input).map_err(|e| e.to_string())?;
    Ok(input)
}

struct WinRtDfuTransport {
    device: BluetoothLEDevice,
    service: GattDeviceService,
    session: GattSession,
    cp: GattCharacteristic,
    pkt: GattCharacteristic,
    notifications: tokio::sync::mpsc::UnboundedReceiver<BleNotification>,
    cp_notify_token: i64,
}

impl Drop for WinRtDfuTransport {
    fn drop(&mut self) {
        let _ = self.cp.RemoveValueChanged(self.cp_notify_token);
        let _ = self.session.Close();
        let _ = self.service.Close();
        let _ = self.device.Close();
    }
}

async fn open_winrt_dfu_transport(peripheral_id: &str) -> Result<WinRtDfuTransport, String> {
    let address = parse_ble_address_u64(peripheral_id)?;
    let open_start = Instant::now();
    let device = BluetoothLEDevice::FromBluetoothAddressAsync(address)
        .map_err(|e| e.to_string())?
        .await
        .map_err(|e| e.to_string())?;
    eprintln!(
        "[ble_ota][winrt] device opened: address={peripheral_id} elapsed_ms={}",
        open_start.elapsed().as_millis()
    );

    if let Ok(preferred) = BluetoothLEPreferredConnectionParameters::ThroughputOptimized() {
        let _ = device.RequestPreferredConnectionParameters(&preferred);
        eprintln!("[ble_ota][winrt] requested throughput optimized connection parameters");
    }

    let service_uuid = nordic_dfu_uuid(0x1530);
    let cp_uuid = nordic_dfu_uuid(0x1531);
    let pkt_uuid = nordic_dfu_uuid(0x1532);
    let ver_uuid = nordic_dfu_uuid(0x1534);

    let service_start = Instant::now();
    let service = first_service_by_uuid(&device, service_uuid).await?;
    eprintln!(
        "[ble_ota][winrt] dfu service ready: elapsed_ms={}",
        service_start.elapsed().as_millis()
    );

    let session = service.Session().map_err(|e| e.to_string())?;
    let _ = session.SetMaintainConnection(true);
    let max_pdu = session.MaxPduSize().map_err(|e| e.to_string())?;
    eprintln!(
        "[ble_ota][winrt] session info: max_pdu_size={} can_maintain_connection={} maintain_connection={}",
        max_pdu,
        session.CanMaintainConnection().ok().unwrap_or(false),
        session.MaintainConnection().ok().unwrap_or(false)
    );

    let chars_start = Instant::now();
    let cp = first_characteristic_by_uuid(&service, cp_uuid, "backend_errors.ble_ota_cp_missing").await?;
    let pkt = first_characteristic_by_uuid(&service, pkt_uuid, "backend_errors.ble_ota_pkt_missing").await?;
    eprintln!(
        "[ble_ota][winrt] characteristics ready: elapsed_ms={}",
        chars_start.elapsed().as_millis()
    );

    if let Ok(ver_result) = service
        .GetCharacteristicsForUuidWithCacheModeAsync(uuid_to_guid(ver_uuid), BluetoothCacheMode::Uncached)
        .map_err(|e| e.to_string())?
        .await
        .map_err(|e| e.to_string())
    {
        if ver_result.Status().ok() == Some(GattCommunicationStatus::Success) {
            let ver_char = if let Ok(chars) = ver_result.Characteristics() {
                if chars.Size().ok().unwrap_or(0) > 0 {
                    chars.GetAt(0).ok()
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(ver_char) = ver_char {
                if let Ok(version_bytes) = read_characteristic_uncached(&ver_char).await {
                    eprintln!("[ble_ota][winrt] dfu version bytes={version_bytes:02X?}");
                }
            }
        }
    }

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let cp_uuid_copy = cp_uuid;
    let token = cp
        .ValueChanged(&TypedEventHandler::new(
            move |_: Ref<GattCharacteristic>, args: Ref<GattValueChangedEventArgs>| {
                if let Ok(args) = args.ok() {
                    let value = args.CharacteristicValue()?;
                    let reader = DataReader::FromBuffer(&value)?;
                    let len = reader.UnconsumedBufferLength()? as usize;
                    let mut input = vec![0u8; len];
                    reader.ReadBytes(&mut input)?;
                    let _ = tx.send(BleNotification {
                        uuid: cp_uuid_copy,
                        value: input,
                    });
                }
                Ok(())
            },
        ))
        .map_err(|e| e.to_string())?;

    let notify_start = Instant::now();
    winrt_set_notify(&cp, true).await?;
    eprintln!(
        "[ble_ota][winrt] notifications enabled: elapsed_ms={}",
        notify_start.elapsed().as_millis()
    );

    Ok(WinRtDfuTransport {
        device,
        service,
        session,
        cp,
        pkt,
        notifications: rx,
        cp_notify_token: token,
    })
}

#[tauri::command]
pub async fn start_ble_ota(
    app: AppHandle,
    state: State<'_, BleOtaJobState>,
    adapter_state: State<'_, BleAdapterState>,
    peripheral_id: String,
    zip_bytes: Vec<u8>,
) -> Result<(), String> {
    let _guard = lock_ble_ota_job(&state)?;
    state.cancel.store(false, Ordering::Release);

    let (bin_data, dat_data, _info) = parse_ota_zip(&zip_bytes)?;
    let _peripheral = find_peripheral_by_id(&adapter_state, &peripheral_id).await?;

    let cancel_flag = Arc::clone(&state.cancel);

    let app_clone = app.clone();
    let result = async move {
        let ota_start = Instant::now();
        emit_ble_ota_progress(
            &app_clone,
            "connecting",
            2,
            i18n_error("ble_ota.progress_connecting"),
            None,
            None,
        );

        let cp_uuid = nordic_dfu_uuid(0x1531);

        emit_ble_ota_progress(
            &app_clone,
            "starting",
            5,
            i18n_error("ble_ota.progress_subscribing"),
            None,
            None,
        );

        let mut transport = open_winrt_dfu_transport(&peripheral_id).await?;
        let cp = transport.cp.clone();
        let pkt = transport.pkt.clone();
        let notifications = &mut transport.notifications;

        let chunk_len = packet_chunk_len();
        eprintln!(
            "[ble_ota][timing] payload prepared: bin_size={} dat_size={} chunk_len={} prn_interval={}",
            bin_data.len(),
            dat_data.len(),
            chunk_len,
            DFU_PRN_INTERVAL
        );

        emit_ble_ota_progress(
            &app_clone,
            "starting",
            8,
            i18n_error("ble_ota.progress_start_dfu"),
            None,
            None,
        );

        // Start DFU (application only)
        let start_req_start = Instant::now();
        winrt_write(&cp, &[OP_START, UPLOAD_MODE_APPLICATION], GattWriteOption::WriteWithResponse, "op_start")
            .await?;
        eprintln!(
            "[ble_ota][timing] write OP_START done: elapsed_ms={}",
            start_req_start.elapsed().as_millis()
        );

        let app_size = bin_data.len() as u32;
        let mut sizes = [0u8; 12];
        sizes[8..12].copy_from_slice(&app_size.to_le_bytes());
        let size_write_start = Instant::now();
        winrt_write(&pkt, &sizes, GattWriteOption::WriteWithoutResponse, "image_size").await?;
        eprintln!(
            "[ble_ota][timing] write image size done: elapsed_ms={}",
            size_write_start.elapsed().as_millis()
        );

        let start_resp_start = Instant::now();
        wait_cp_response(
            notifications,
            cp_uuid,
            OP_START,
            Duration::from_secs(120),
        )
        .await?;
        eprintln!(
            "[ble_ota][timing] wait OP_START response done: elapsed_ms={}",
            start_resp_start.elapsed().as_millis()
        );

        emit_ble_ota_progress(
            &app_clone,
            "init",
            12,
            i18n_error("ble_ota.progress_init"),
            None,
            None,
        );

        let init_begin_start = Instant::now();
        winrt_write(&cp, &[OP_INIT, 0x00], GattWriteOption::WriteWithResponse, "op_init_begin")
            .await?;
        eprintln!(
            "[ble_ota][timing] write OP_INIT begin done: elapsed_ms={}",
            init_begin_start.elapsed().as_millis()
        );

        let init_data_start = Instant::now();
        write_packet_chunks(
            &pkt,
            &dat_data,
            chunk_len,
            cancel_flag.as_ref(),
        )
        .await?;
        eprintln!(
            "[ble_ota][timing] init packet transfer done: elapsed_ms={}",
            init_data_start.elapsed().as_millis()
        );

        let init_end_start = Instant::now();
        winrt_write(&cp, &[OP_INIT, 0x01], GattWriteOption::WriteWithResponse, "op_init_end")
            .await?;
        eprintln!(
            "[ble_ota][timing] write OP_INIT end done: elapsed_ms={}",
            init_end_start.elapsed().as_millis()
        );

        let init_resp_start = Instant::now();
        wait_cp_response(
            notifications,
            cp_uuid,
            OP_INIT,
            Duration::from_secs(60),
        )
        .await?;
        eprintln!(
            "[ble_ota][timing] wait OP_INIT response done: elapsed_ms={}",
            init_resp_start.elapsed().as_millis()
        );

        // PRN：小端 N。这里对齐 nRF Connect Android 在该设备上的默认行为：4 包一通知。
        let prn_n = DFU_PRN_INTERVAL.to_le_bytes();
        let prn_cfg_start = Instant::now();
        winrt_write(&cp, &[OP_PRN, prn_n[0], prn_n[1]], GattWriteOption::WriteWithResponse, "op_prn")
            .await?;
        eprintln!(
            "[ble_ota][timing] write OP_PRN done: elapsed_ms={}",
            prn_cfg_start.elapsed().as_millis()
        );

        let receive_fw_start = Instant::now();
        winrt_write(&cp, &[OP_RECEIVE_FW], GattWriteOption::WriteWithResponse, "op_receive_fw")
            .await?;
        eprintln!(
            "[ble_ota][timing] write OP_RECEIVE_FW done: elapsed_ms={}",
            receive_fw_start.elapsed().as_millis()
        );

        emit_ble_ota_progress(
            &app_clone,
            "transferring",
            15,
            i18n_error("ble_ota.progress_transferring"),
            Some(0),
            Some(bin_data.len() as u64),
        );

        let firmware_chunks = chunk_len.max(4);
        let total = bin_data.len() as u64;
        eprintln!(
            "[ble_ota] transfer start: bin_size={} chunk_len={} prn_interval={}",
            total, firmware_chunks, DFU_PRN_INTERVAL
        );
        let firmware_start = Instant::now();
        let mut offset = 0usize;
        let prn_every = u32::from(DFU_PRN_INTERVAL);
        let mut last_progress_pct: u8 = 15;
        let mut packets_until_prn = prn_every.max(1);
        let mut transfer_completed = false;
        let mut firmware_write_calls: u64 = 0;
        let mut firmware_write_bytes: u64 = 0;
        let mut firmware_write_total = Duration::ZERO;
        let mut firmware_write_max = Duration::ZERO;
        let mut prn_wait_count: u64 = 0;
        let mut prn_wait_total = Duration::ZERO;
        let mut prn_wait_max = Duration::ZERO;
        while offset < bin_data.len() {
            if cancel_flag.as_ref().load(Ordering::Acquire) {
                return Err(i18n_error("backend_errors.ble_ota_cancelled"));
            }

            let end = (offset + firmware_chunks).min(bin_data.len());
            let slice = &bin_data[offset..end];

            let pkt_write_start = Instant::now();
            winrt_write(&pkt, slice, GattWriteOption::WriteWithoutResponse, "firmware_chunk")
                .await?;
            let pkt_write_elapsed = pkt_write_start.elapsed();
            firmware_write_calls += 1;
            firmware_write_bytes += slice.len() as u64;
            firmware_write_total += pkt_write_elapsed;
            firmware_write_max = firmware_write_max.max(pkt_write_elapsed);

            offset = end;
            let sent = offset as u64;
            packets_until_prn = packets_until_prn.saturating_sub(1);

            // 更接近 nRF Connect Android 的 credit 模型：
            // 连续发送 N 包后等待一个 PRN，再恢复下一轮额度；最后一包后等待完成响应。
            let reached_end = offset >= bin_data.len();
            if packets_until_prn == 0 || reached_end {
                let wait_start = Instant::now();
                let confirmed = match wait_firmware_transfer_event(
                    notifications,
                    cp_uuid,
                    sent as u32,
                    if reached_end {
                        Duration::from_secs(120)
                    } else {
                        Duration::from_secs(30)
                    },
                )
                .await?
                {
                    FirmwareTransferEvent::Prn(received) => {
                        let wait_elapsed = wait_start.elapsed();
                        prn_wait_count += 1;
                        prn_wait_total += wait_elapsed;
                        prn_wait_max = prn_wait_max.max(wait_elapsed);
                        packets_until_prn = prn_every.max(1);
                        u64::from(received).min(total)
                    }
                    FirmwareTransferEvent::Complete => {
                        let wait_elapsed = wait_start.elapsed();
                        prn_wait_count += 1;
                        prn_wait_total += wait_elapsed;
                        prn_wait_max = prn_wait_max.max(wait_elapsed);
                        transfer_completed = true;
                        total
                    }
                };

                if prn_wait_count <= 5 || prn_wait_count % 25 == 0 || transfer_completed {
                    let avg_write_ms = if firmware_write_calls > 0 {
                        firmware_write_total.as_secs_f64() * 1000.0 / firmware_write_calls as f64
                    } else {
                        0.0
                    };
                    let avg_prn_wait_ms = if prn_wait_count > 0 {
                        prn_wait_total.as_secs_f64() * 1000.0 / prn_wait_count as f64
                    } else {
                        0.0
                    };
                    eprintln!(
                        "[ble_ota][timing] fw progress: sent={}/{} packets={} prn_events={} avg_write_ms={:.2} max_write_ms={} avg_wait_ms={:.2} max_wait_ms={}",
                        confirmed,
                        total,
                        firmware_write_calls,
                        prn_wait_count,
                        avg_write_ms,
                        firmware_write_max.as_millis(),
                        avg_prn_wait_ms,
                        prn_wait_max.as_millis()
                    );
                }

                let progress_pct = (15u64 + (confirmed * 70 / total.max(1))) as u8;
                let clamped = progress_pct.min(85);
                if clamped != last_progress_pct {
                    last_progress_pct = clamped;
                    emit_ble_ota_progress(
                        &app_clone,
                        "transferring",
                        clamped,
                        i18n_error("ble_ota.progress_transferring"),
                        Some(confirmed),
                        Some(total),
                    );
                }
            }
        }

        if !transfer_completed {
            loop {
                let tail_wait_start = Instant::now();
                match wait_firmware_transfer_event(
                    notifications,
                    cp_uuid,
                    total as u32,
                    Duration::from_secs(120),
                )
                .await?
                {
                    FirmwareTransferEvent::Prn(received) => {
                        let wait_elapsed = tail_wait_start.elapsed();
                        prn_wait_count += 1;
                        prn_wait_total += wait_elapsed;
                        prn_wait_max = prn_wait_max.max(wait_elapsed);
                        let confirmed = u64::from(received).min(total);
                        let progress_pct = (15u64 + (confirmed * 70 / total.max(1))) as u8;
                        let clamped = progress_pct.min(85);
                        if clamped != last_progress_pct {
                            last_progress_pct = clamped;
                            emit_ble_ota_progress(
                                &app_clone,
                                "transferring",
                                clamped,
                                i18n_error("ble_ota.progress_transferring"),
                                Some(confirmed),
                                Some(total),
                            );
                        }
                        eprintln!(
                            "[ble_ota][timing] tail PRN: confirmed={}/{} wait_ms={}",
                            confirmed,
                            total,
                            wait_elapsed.as_millis()
                        );
                    }
                    FirmwareTransferEvent::Complete => {
                        let wait_elapsed = tail_wait_start.elapsed();
                        prn_wait_count += 1;
                        prn_wait_total += wait_elapsed;
                        prn_wait_max = prn_wait_max.max(wait_elapsed);
                        eprintln!(
                            "[ble_ota][timing] firmware completion response received: wait_ms={}",
                            wait_elapsed.as_millis()
                        );
                        break;
                    }
                }
            }
        }
        eprintln!(
            "[ble_ota][timing] firmware transfer summary: elapsed_ms={} write_calls={} bytes={} avg_write_ms={:.2} max_write_ms={} prn_events={} avg_wait_ms={:.2} max_wait_ms={}",
            firmware_start.elapsed().as_millis(),
            firmware_write_calls,
            firmware_write_bytes,
            if firmware_write_calls > 0 {
                firmware_write_total.as_secs_f64() * 1000.0 / firmware_write_calls as f64
            } else {
                0.0
            },
            firmware_write_max.as_millis(),
            prn_wait_count,
            if prn_wait_count > 0 {
                prn_wait_total.as_secs_f64() * 1000.0 / prn_wait_count as f64
            } else {
                0.0
            },
            prn_wait_max.as_millis()
        );

        emit_ble_ota_progress(
            &app_clone,
            "validating",
            88,
            i18n_error("ble_ota.progress_validating"),
            Some(total),
            Some(total),
        );

        let validate_start = Instant::now();
        winrt_write(&cp, &[OP_VALIDATE], GattWriteOption::WriteWithResponse, "op_validate")
            .await?;
        eprintln!(
            "[ble_ota][timing] write OP_VALIDATE done: elapsed_ms={}",
            validate_start.elapsed().as_millis()
        );

        let validate_resp_start = Instant::now();
        wait_cp_response(
            notifications,
            cp_uuid,
            OP_VALIDATE,
            Duration::from_secs(120),
        )
        .await?;
        eprintln!(
            "[ble_ota][timing] wait OP_VALIDATE response done: elapsed_ms={}",
            validate_resp_start.elapsed().as_millis()
        );

        emit_ble_ota_progress(
            &app_clone,
            "activating",
            95,
            i18n_error("ble_ota.progress_activating"),
            None,
            None,
        );

        let activate_start = Instant::now();
        winrt_write(&cp, &[OP_ACTIVATE], GattWriteOption::WriteWithResponse, "op_activate")
            .await?;
        eprintln!(
            "[ble_ota][timing] write OP_ACTIVATE done: elapsed_ms={}",
            activate_start.elapsed().as_millis()
        );

        let _ = winrt_set_notify(&cp, false).await;
        drop(transport);
        eprintln!(
            "[ble_ota][timing] ota finished: total_elapsed_ms={}",
            ota_start.elapsed().as_millis()
        );

        emit_ble_ota_progress(
            &app_clone,
            "success",
            100,
            i18n_error("ble_ota.progress_success"),
            Some(total),
            Some(total),
        );

        Ok::<(), String>(())
    }
    .await;

    if let Err(ref e) = result {
        emit_ble_ota_progress(
            &app,
            "error",
            0,
            e.clone(),
            None,
            None,
        );
    }

    state.cancel.store(false, Ordering::Release);
    result
}

async fn write_packet_chunks(
    pkt: &GattCharacteristic,
    data: &[u8],
    chunk_len: usize,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let chunk_len = chunk_len.max(4);
    let mut offset = 0usize;
    let start = Instant::now();
    let mut write_calls: u64 = 0;
    let mut write_total = Duration::ZERO;
    let mut write_max = Duration::ZERO;
    while offset < data.len() {
        if cancel.load(Ordering::Acquire) {
            return Err(i18n_error("backend_errors.ble_ota_cancelled"));
        }
        let end = (offset + chunk_len).min(data.len());
        let slice = &data[offset..end];
        let write_start = Instant::now();
        winrt_write(pkt, slice, GattWriteOption::WriteWithoutResponse, "packet_chunk").await?;
        let write_elapsed = write_start.elapsed();
        write_calls += 1;
        write_total += write_elapsed;
        write_max = write_max.max(write_elapsed);
        offset = end;
    }
    eprintln!(
        "[ble_ota][timing] packet chunk transfer summary: bytes={} chunk_len={} write_calls={} elapsed_ms={} avg_write_ms={:.2} max_write_ms={}",
        data.len(),
        chunk_len,
        write_calls,
        start.elapsed().as_millis(),
        if write_calls > 0 {
            write_total.as_secs_f64() * 1000.0 / write_calls as f64
        } else {
            0.0
        },
        write_max.as_millis()
    );
    Ok(())
}

#[tauri::command]
pub fn cancel_ble_ota(state: State<'_, BleOtaJobState>) -> Result<(), String> {
    state.cancel.store(true, Ordering::Release);
    Ok(())
}
