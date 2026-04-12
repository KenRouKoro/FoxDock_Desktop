//! 接收器 HID_1 单行命令通道（Report ID 2 OUT / 3 IN，支持多帧结果）。
//! 协议见 `docs/receiverr/usb-hid-host-integration.zh.md` §6.3。

use serde::Serialize;
use serde_json::json;

#[cfg(windows)]
use hidapi::{DeviceInfo, HidApi};
#[cfg(windows)]
use std::collections::HashMap;
#[cfg(windows)]
use std::sync::Mutex;
#[cfg(windows)]
use std::time::{Duration, Instant};

#[cfg(windows)]
use crate::i18n_error;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiverHidCommandResult {
    pub ok: bool,
    pub status_code: u8,
    /// 按 `chunk_idx` 升序拼接后的完整输出文本（与 CDC printk 等价）。
    pub message: String,
    pub line: String,
    #[serde(default)]
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_total: Option<u8>,
}

#[cfg(windows)]
const RECEIVER_VID: u16 = 0x1209;
#[cfg(windows)]
const RECEIVER_PID: u16 = 0x7690;
#[cfg(windows)]
const HID_1_IFACE: i32 = 3;
#[cfg(windows)]
const RPT_CMD_OUT: u8 = 2;
#[cfg(windows)]
const RPT_CMD_STATUS: u8 = 3;
#[cfg(windows)]
const RPT_SIZE: usize = 64;
#[cfg(windows)]
const MAX_CMD_PAYLOAD: usize = 61;
#[cfg(windows)]
const MAX_R3_PAYLOAD: usize = 56;

/// `flags`：§6.3（FIRST 由固件标注；重组以 chunk_idx / LAST 为准）
#[cfg(windows)]
const FLAG_LAST: u8 = 1 << 1;
#[cfg(windows)]
const FLAG_TRUNCATED: u8 = 1 << 2;

#[cfg(windows)]
static SERIAL_SEND: Mutex<()> = Mutex::new(());

#[cfg(windows)]
fn next_seq() -> u8 {
    use std::sync::atomic::{AtomicU8, Ordering};
    static SEQ: AtomicU8 = AtomicU8::new(1);
    SEQ.fetch_add(1, Ordering::Relaxed)
}

#[cfg(windows)]
struct MultiframeAccumulator {
    expect_seq: u8,
    status_code: Option<u8>,
    chunk_total: Option<u8>,
    /// chunk_idx -> payload bytes（长度 = 该帧 payload_len）
    chunks: HashMap<u8, Vec<u8>>,
    truncated: bool,
}

#[cfg(windows)]
impl MultiframeAccumulator {
    fn new(expect_seq: u8) -> Self {
        Self {
            expect_seq,
            status_code: None,
            chunk_total: None,
            chunks: HashMap::new(),
            truncated: false,
        }
    }

    fn ingest_frame(
        &mut self,
        buf: &[u8],
        n: usize,
        log: &mut dyn FnMut(&str),
    ) -> Result<Option<ReceiverHidCommandResult>, String> {
        if n < 8 {
            return Err(i18n_error("backend_errors.receiver_hid_bad_frame"));
        }
        if buf[0] != RPT_CMD_STATUS {
            return Err(i18n_error("backend_errors.receiver_hid_bad_frame"));
        }
        if buf[1] != self.expect_seq {
            return Err(i18n_error("backend_errors.receiver_hid_bad_frame"));
        }

        let status = buf[2];
        let flags = buf[3];
        let chunk_idx = buf[4];
        let chunk_total = buf[5];
        let payload_len = buf[6] as usize;
        let _reserved = buf[7];

        if chunk_total == 0 {
            return Err(i18n_error("backend_errors.receiver_hid_bad_frame"));
        }
        if payload_len > MAX_R3_PAYLOAD {
            return Err(i18n_error("backend_errors.receiver_hid_bad_frame"));
        }
        if chunk_idx >= chunk_total {
            return Err(i18n_error("backend_errors.receiver_hid_bad_frame"));
        }
        if n < 8 + payload_len {
            return Err(i18n_error("backend_errors.receiver_hid_bad_frame"));
        }

        if let Some(prev) = self.status_code {
            if prev != status {
                log(&format!(
                    "receiver HID cmd: warning status_code mismatch prev={prev} frame={status}\n"
                ));
            }
        } else {
            self.status_code = Some(status);
        }

        if let Some(ct) = self.chunk_total {
            if ct != chunk_total {
                return Err(i18n_error("backend_errors.receiver_hid_bad_frame"));
            }
        } else {
            self.chunk_total = Some(chunk_total);
        }

        let payload = buf[8..8 + payload_len].to_vec();
        if self.chunks.insert(chunk_idx, payload).is_some() {
            return Err(i18n_error("backend_errors.receiver_hid_duplicate_chunk"));
        }

        if (flags & FLAG_TRUNCATED) != 0 {
            self.truncated = true;
        }

        log(&format!(
            "receiver HID cmd: IN seq={} st={} fl=0x{:02x} chunk={}/{} pay_len={}\n",
            self.expect_seq, status, flags, chunk_idx, chunk_total, payload_len
        ));

        if (flags & FLAG_LAST) != 0 {
            if chunk_idx != chunk_total.saturating_sub(1) {
                return Err(i18n_error("backend_errors.receiver_hid_bad_frame"));
            }
            return Ok(Some(self.finalize(status)?));
        }
        Ok(None)
    }

    fn finalize(&self, status_from_last: u8) -> Result<ReceiverHidCommandResult, String> {
        let chunk_total = self.chunk_total.ok_or_else(|| {
            i18n_error("backend_errors.receiver_hid_incomplete_frames")
        })?;
        let st = self.status_code.unwrap_or(status_from_last);
        let mut assembled = Vec::new();
        for i in 0..chunk_total {
            let Some(p) = self.chunks.get(&i) else {
                return Err(i18n_error("backend_errors.receiver_hid_incomplete_frames"));
            };
            assembled.extend_from_slice(p);
        }
        let message = String::from_utf8_lossy(&assembled).to_string();
        let ok = st == 0;
        Ok(ReceiverHidCommandResult {
            ok,
            status_code: st,
            message,
            line: String::new(),
            truncated: self.truncated,
            chunk_total: Some(chunk_total),
        })
    }
}

#[cfg(windows)]
fn read_multiframe_response(
    device: &mut hidapi::HidDevice,
    expect_seq: u8,
    timeout_ms: u32,
    line: &str,
    log: &mut dyn FnMut(&str),
) -> Result<ReceiverHidCommandResult, String> {
    let mut buf = [0u8; RPT_SIZE];
    let deadline = Instant::now() + Duration::from_millis(timeout_ms.max(100) as u64);
    let mut acc = MultiframeAccumulator::new(expect_seq);

    loop {
        if Instant::now() >= deadline {
            return Err(i18n_error("backend_errors.receiver_hid_read_timeout"));
        }

        let remaining_ms = deadline
            .saturating_duration_since(Instant::now())
            .as_millis()
            .min(500) as i32;
        if remaining_ms <= 0 {
            return Err(i18n_error("backend_errors.receiver_hid_read_timeout"));
        }

        let n = device.read_timeout(&mut buf, remaining_ms).map_err(|e| {
            crate::i18n_error_with_params(
                "backend_errors.receiver_hid_read_failed",
                json!({ "error": e.to_string() }),
            )
        })?;

        if n == 0 {
            continue;
        }

        if n < 3 {
            continue;
        }
        if buf[0] != RPT_CMD_STATUS {
            continue;
        }
        if buf[1] != expect_seq {
            continue;
        }

        match acc.ingest_frame(&buf[..n.min(RPT_SIZE)], n.min(RPT_SIZE), log) {
            Ok(Some(mut res)) => {
                res.line = line.to_string();
                return Ok(res);
            }
            Ok(None) => {}
            Err(e) => return Err(e),
        }
    }
}

/// 向接收器 HID_1 发送一行控制台命令并读取多帧状态报告。
#[cfg(windows)]
pub fn send_receiver_hid_console_line_with_log(
    serial_hint: Option<&str>,
    line: &str,
    timeout_ms: u32,
    log: &mut dyn FnMut(&str),
) -> Result<ReceiverHidCommandResult, String> {
    let _guard = SERIAL_SEND
        .lock()
        .map_err(|_| i18n_error("backend_errors.internal_state_corrupted"))?;

    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err(i18n_error("backend_errors.receiver_hid_empty_line"));
    }
    let bytes = trimmed.as_bytes();
    if bytes.len() > MAX_CMD_PAYLOAD {
        return Err(i18n_error("backend_errors.receiver_hid_line_too_long"));
    }

    let api = HidApi::new().map_err(|e| {
        crate::i18n_error_with_params(
            "backend_errors.receiver_hid_hidapi_failed",
            json!({ "error": e.to_string() }),
        )
    })?;

    let target_key = serial_hint
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_uppercase());

    let mut candidates: Vec<&DeviceInfo> = api
        .device_list()
        .filter(|d| d.vendor_id() == RECEIVER_VID && d.product_id() == RECEIVER_PID)
        .collect();

    if candidates.is_empty() {
        return Err(i18n_error("backend_errors.receiver_hid_no_device"));
    }

    candidates.sort_by_key(|d| {
        if d.interface_number() == HID_1_IFACE {
            0
        } else {
            1
        }
    });

    if let Some(ref key) = target_key {
        let mut serial_matches: Vec<&DeviceInfo> = candidates
            .iter()
            .copied()
            .filter(|d| {
                d.serial_number()
                    .map(|s| s.eq_ignore_ascii_case(key.trim()))
                    .unwrap_or(false)
            })
            .collect();
        if !serial_matches.is_empty() {
            serial_matches.sort_by_key(|d| {
                if d.interface_number() == HID_1_IFACE {
                    0
                } else {
                    1
                }
            });
            candidates = serial_matches;
        }
    }

    for d in candidates {
        let path = d.path().to_string_lossy();
        log(&format!(
            "receiver HID cmd: trying iface={} path={}\n",
            d.interface_number(),
            path
        ));

        let mut device = match api.open_path(d.path()) {
            Ok(dev) => dev,
            Err(e) => {
                log(&format!("receiver HID cmd: open_path failed: {e}\n"));
                continue;
            }
        };

        let seq = next_seq();
        let mut out = [0u8; RPT_SIZE];
        out[0] = RPT_CMD_OUT;
        out[1] = seq;
        out[2] = bytes.len() as u8;
        out[3..3 + bytes.len()].copy_from_slice(bytes);

        match device.write(&out) {
            Ok(n) => log(&format!(
                "receiver HID cmd: write OUT seq={seq} payload_len={} -> {n} byte(s)\n",
                bytes.len()
            )),
            Err(e) => {
                log(&format!("receiver HID cmd: write failed: {e}\n"));
                continue;
            }
        }

        match read_multiframe_response(&mut device, seq, timeout_ms, trimmed, log) {
            Ok(mut res) => {
                res.line = trimmed.to_string();
                log(&format!(
                    "receiver HID cmd: done ok={} status={} truncated={} chunks={:?} text_len={}\n",
                    res.ok,
                    res.status_code,
                    res.truncated,
                    res.chunk_total,
                    res.message.len()
                ));
                return Ok(res);
            }
            Err(e) => {
                log(&format!("receiver HID cmd: read/assemble failed: {e}\n"));
                continue;
            }
        }
    }

    Err(i18n_error("backend_errors.receiver_hid_no_response"))
}

#[cfg(not(windows))]
pub fn send_receiver_hid_console_line_with_log(
    _serial_hint: Option<&str>,
    line: &str,
    _timeout_ms: u32,
    _log: &mut dyn FnMut(&str),
) -> Result<ReceiverHidCommandResult, String> {
    let _ = line;
    Err(crate::i18n_error("backend_errors.windows_only"))
}
