//! USB 构建信息读取（当前 Windows 使用 HID Feature Report）。
//!
//! 文档约定见 `docs/USB_BUILD_INFO_HID_DESKTOP.md`。

#[cfg(windows)]
use hidapi::{DeviceInfo, HidApi};
#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use windows_sys::Win32::Devices::HumanInterfaceDevice::HidD_GetFeature;
#[cfg(windows)]
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE,
};
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};

const TRACKER_VID: u16 = 0x1209;
const TRACKER_PID: u16 = 0x7692;
const BUILD_INFO_REPORT_ID: u8 = 0x01;
const BUILD_INFO_PAYLOAD_LEN: usize = 24;
const BUILD_INFO_REPORT_LEN: usize = 1 + BUILD_INFO_PAYLOAD_LEN;
const BUILD_INFO_INTERFACE_NUMBER: i32 = 3;
const FEATURE_REPORT_ATTEMPT_LENS: [usize; 4] = [BUILD_INFO_REPORT_LEN, 32, 64, 65];

/// 尽力通过 nusb 读取追踪器固件版本；失败时返回 `None`（不影响拓扑扫描）。
#[allow(dead_code)] // 对外保留；应用内优先使用 `read_tracker_usb_build_info_with_log`
pub fn read_tracker_usb_build_info(setup_device_id: &str) -> Option<String> {
    read_tracker_usb_build_info_with_log(setup_device_id, |_| {})
}

/// 与 [`read_tracker_usb_build_info`] 相同，但将诊断信息写入 `log`（例如 Tauri 调试台）。
pub fn read_tracker_usb_build_info_with_log(
    setup_device_id: &str,
    mut log: impl FnMut(&str),
) -> Option<String> {
    #[cfg(windows)]
    {
        read_tracker_usb_build_info_impl(setup_device_id, &mut log)
    }
    #[cfg(not(windows))]
    {
        let _ = setup_device_id;
        let _ = log;
        None
    }
}

/// 将固件返回的原始字节格式化为简短展示字符串。
pub fn format_usb_build_info_v1(raw: &[u8]) -> Option<String> {
    if raw.len() < BUILD_INFO_PAYLOAD_LEN {
        return None;
    }
    let major = raw[1];
    let minor = raw[2];
    let patch = raw[3];
    let tweak = u16::from_le_bytes([raw[4], raw[5]]);
    let flags = u16::from_le_bytes([raw[6], raw[7]]);
    let hash_bytes = &raw[12..24];
    let hash = core::str::from_utf8(hash_bytes).ok()?.trim();
    let dirty = (flags & 1) != 0;
    let mut s = format!("{major}.{minor}.{patch}+{tweak} {hash}");
    if dirty {
        s.push_str(" dirty");
    }
    Some(s)
}

/// 从 HID Feature Report 中提取 `usb_build_info_v1` payload（24B）。
pub fn extract_build_info_payload_from_feature_report(report: &[u8]) -> Option<&[u8]> {
    if report.len() < BUILD_INFO_REPORT_LEN || report[0] != BUILD_INFO_REPORT_ID {
        return None;
    }
    Some(&report[1..BUILD_INFO_REPORT_LEN])
}

#[cfg(windows)]
fn try_read_build_info_feature_report(
    device: &hidapi::HidDevice,
    log: &mut dyn FnMut(&str),
) -> Option<Vec<u8>> {
    for len in FEATURE_REPORT_ATTEMPT_LENS {
        let mut report = vec![0u8; len];
        report[0] = BUILD_INFO_REPORT_ID;
        match device.get_feature_report(&mut report) {
            Ok(n) => {
                log(&format!(
                    "USB build info(HID): get_feature_report len={} -> {} byte(s)\n",
                    len, n
                ));
                if let Some(payload) = extract_build_info_payload_from_feature_report(&report[..n]) {
                    return Some(payload.to_vec());
                }
                log("USB build info(HID): report shape did not match expected ID/payload length\n");
            }
            Err(e) => {
                log(&format!(
                    "USB build info(HID): get_feature_report len={} failed: {e}\n",
                    len
                ));
            }
        }
    }
    None
}

#[cfg(windows)]
fn try_read_build_info_via_hidd(
    hid_path: &str,
    log: &mut dyn FnMut(&str),
) -> Option<Vec<u8>> {
    let wide_path: Vec<u16> = OsStr::new(hid_path).encode_wide().chain(Some(0)).collect();
    let handle = unsafe {
        CreateFileW(
            wide_path.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            core::ptr::null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            core::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        let err = unsafe { GetLastError() };
        log(&format!(
            "USB build info(HID): CreateFileW for HidD_GetFeature failed, err={err}\n"
        ));
        return None;
    }

    let mut payload = None;
    for len in FEATURE_REPORT_ATTEMPT_LENS {
        let mut report = vec![0u8; len];
        report[0] = BUILD_INFO_REPORT_ID;
        let ok = unsafe { HidD_GetFeature(handle, report.as_mut_ptr() as _, report.len() as u32) };
        if ok == 0 {
            let err = unsafe { GetLastError() };
            log(&format!(
                "USB build info(HID): HidD_GetFeature len={} failed, err={err}\n",
                len
            ));
            continue;
        }
        log(&format!(
            "USB build info(HID): HidD_GetFeature len={} succeeded\n",
            len
        ));
        if let Some(extracted) = extract_build_info_payload_from_feature_report(&report) {
            payload = Some(extracted.to_vec());
            break;
        }
        log("USB build info(HID): HidD_GetFeature returned unexpected report shape\n");
    }

    unsafe {
        CloseHandle(handle);
    }
    payload
}

/// 去掉硬件 ID 段（`USB\` 与下一级 `\` 之间）中的 `&MI_XX`，便于与复合设备父节点实例 ID 对齐。
fn strip_mi_from_device_id(id: &str) -> String {
    let upper = id.trim().to_ascii_uppercase();
    let Some(first_slash) = upper.find('\\') else {
        return upper;
    };
    let Some(rest_to_second) = upper[first_slash + 1..].find('\\') else {
        return upper;
    };
    let second_slash = first_slash + 1 + rest_to_second;
    let hw_segment = &upper[first_slash + 1..second_slash];
    let cleaned_hw = if let Some(mi_pos) = hw_segment.find("&MI_") {
        let after_mi = &hw_segment[mi_pos + 4..];
        let digit_len = after_mi
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .count();
        let remove_len = 4 + digit_len;
        format!(
            "{}{}",
            &hw_segment[..mi_pos],
            &hw_segment[mi_pos + remove_len..]
        )
    } else {
        hw_segment.to_string()
    };
    format!(
        "{}{}{}",
        &upper[..first_slash + 1],
        cleaned_hw,
        &upper[second_slash..]
    )
}

/// 实例 ID 中第三段（`USB\VID…&PID…\` 之后）常用于与字符串描述符序列号对照。
fn instance_key_from_device_id(device_id: &str) -> Option<String> {
    let cleaned = strip_mi_from_device_id(device_id);
    let parts: Vec<&str> = cleaned
        .split('\\')
        .filter(|s| !s.is_empty())
        .collect();
    if parts.len() < 3 {
        return None;
    }
    Some(parts[2].to_string())
}

#[cfg(windows)]
fn read_tracker_usb_build_info_impl(
    setup_device_id: &str,
    log: &mut dyn FnMut(&str),
) -> Option<String> {
    let target_key = instance_key_from_device_id(setup_device_id).map(|s| s.to_ascii_uppercase());
    log(&format!(
        "USB build info(HID): SetupAPI device_id = {}\n",
        setup_device_id
    ));

    let api = match HidApi::new() {
        Ok(api) => api,
        Err(e) => {
            log(&format!("USB build info(HID): HidApi::new failed: {e}\n"));
            return None;
        }
    };

    let mut candidates: Vec<&DeviceInfo> = api
        .device_list()
        .filter(|d| d.vendor_id() == TRACKER_VID && d.product_id() == TRACKER_PID)
        .collect();
    log(&format!(
        "USB build info(HID): found {} HID device(s) with {:04X}:{:04X}\n",
        candidates.len(),
        TRACKER_VID,
        TRACKER_PID
    ));
    for (i, d) in candidates.iter().take(12).enumerate() {
        let path = d.path().to_string_lossy();
        let serial = d.serial_number().unwrap_or("");
        log(&format!(
            "  [{}] iface={} usage_page=0x{:04X} usage=0x{:04X}\n      serial={} path={}\n",
            i,
            d.interface_number(),
            d.usage_page(),
            d.usage(),
            serial,
            path
        ));
    }
    if candidates.len() > 12 {
        log("  ... (truncated)\n");
    }

    candidates.sort_by_key(|d| {
        if d.interface_number() == BUILD_INFO_INTERFACE_NUMBER {
            0
        } else {
            1
        }
    });

    if let Some(key) = target_key {
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
                if d.interface_number() == BUILD_INFO_INTERFACE_NUMBER {
                    0
                } else {
                    1
                }
            });
            log(&format!(
                "USB build info(HID): narrowed by serial/key `{}` => {} candidate(s)\n",
                key,
                serial_matches.len()
            ));
            candidates = serial_matches;
        }
    }

    for d in candidates {
        let path = d.path().to_string_lossy();
        log(&format!(
            "USB build info(HID): trying iface={} path={}\n",
            d.interface_number(),
            path
        ));

        let device = match api.open_path(d.path()) {
            Ok(dev) => dev,
            Err(e) => {
                log(&format!("USB build info(HID): open_path failed: {e}\n"));
                continue;
            }
        };

        let hid_path = d.path().to_string_lossy().into_owned();
        let payload = try_read_build_info_feature_report(&device, log)
            .or_else(|| try_read_build_info_via_hidd(&hid_path, log));
        let Some(payload) = payload else {
            continue;
        };

        if let Some(version) = format_usb_build_info_v1(&payload) {
            log(&format!(
                "USB build info(HID): OK via iface={} path={}\n",
                d.interface_number(),
                path
            ));
            return Some(version);
        }
        log("USB build info(HID): payload parse failed\n");
    }
    log("USB build info(HID): no HID candidate produced valid build info\n");
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_mi_removes_interface_suffix() {
        let with_mi = r"USB\VID_1209&PID_7692&MI_00\6&ABCDEF&0&5";
        let without = r"USB\VID_1209&PID_7692\6&ABCDEF&0&5";
        assert_eq!(strip_mi_from_device_id(with_mi), without.to_ascii_uppercase());
    }

    #[test]
    fn format_sample_build_info() {
        let mut raw = [0u8; 24];
        raw[0] = 1;
        raw[1] = 1;
        raw[2] = 2;
        raw[3] = 3;
        raw[4] = 0x34;
        raw[5] = 0x12;
        raw[6] = 0;
        raw[7] = 0;
        raw[8..12].copy_from_slice(&0x12345678u32.to_le_bytes());
        raw[12..24].copy_from_slice(b"deadbeefcafe");
        let s = format_usb_build_info_v1(&raw).expect("format");
        assert!(s.contains("1.2.3+4660"));
        assert!(s.contains("deadbeefcafe"));
    }

    #[test]
    fn extract_feature_report_payload() {
        let mut report = [0u8; BUILD_INFO_REPORT_LEN];
        report[0] = BUILD_INFO_REPORT_ID;
        report[1] = 1;
        report[2] = 2;
        let payload = extract_build_info_payload_from_feature_report(&report).expect("payload");
        assert_eq!(payload.len(), BUILD_INFO_PAYLOAD_LEN);
        assert_eq!(payload[0], 1);
        assert_eq!(payload[1], 2);
    }

    #[test]
    fn reject_invalid_feature_report() {
        let invalid_report_id = [0u8; BUILD_INFO_REPORT_LEN];
        assert!(extract_build_info_payload_from_feature_report(&invalid_report_id).is_none());
        let too_short = [BUILD_INFO_REPORT_ID, 1, 2];
        assert!(extract_build_info_payload_from_feature_report(&too_short).is_none());
    }

    #[test]
    fn extract_feature_report_payload_allows_larger_buffers() {
        let mut report = [0u8; 64];
        report[0] = BUILD_INFO_REPORT_ID;
        report[1] = 9;
        report[24] = 7;
        let payload = extract_build_info_payload_from_feature_report(&report).expect("payload");
        assert_eq!(payload.len(), BUILD_INFO_PAYLOAD_LEN);
        assert_eq!(payload[0], 9);
        assert_eq!(payload[23], 7);
    }
}
