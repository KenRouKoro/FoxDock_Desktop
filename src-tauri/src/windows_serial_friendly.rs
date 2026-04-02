//! Windows-only: map COM ports (Slime Smol Tracker / Receiver USB serial) to
//! Bus Reported Device Description, with SPDRP_FRIENDLYNAME as fallback.

use std::collections::HashMap;

use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW,
    SetupDiGetDevicePropertyW, SetupDiGetDeviceRegistryPropertyW, SetupDiOpenDevRegKey,
    DICS_FLAG_GLOBAL, DIGCF_PRESENT, DIREG_DEV, HDEVINFO, SP_DEVINFO_DATA,
    SPDRP_FRIENDLYNAME, SPDRP_HARDWAREID, GUID_DEVCLASS_PORTS,
};
use windows_sys::Win32::Devices::Properties::{
    DEVPKEY_Device_BusReportedDeviceDesc, DEVPROP_TYPE_STRING, DEVPROPTYPE,
};
use windows_sys::Win32::Foundation::{
    GetLastError, ERROR_INSUFFICIENT_BUFFER, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS,
    INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::System::Registry::{RegCloseKey, RegQueryValueExW, HKEY, KEY_READ, REG_SZ};

const SLIME_SMOL_VID: &str = "VID_1209";
const SLIME_SMOL_TRACKER_PID: &str = "PID_7692";
const SLIME_SMOL_RECEIVER_PID: &str = "PID_7690";

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

fn get_device_property_string(
    hdev: HDEVINFO,
    devinfo: &SP_DEVINFO_DATA,
    key: &windows_sys::Win32::Devices::Properties::DEVPROPKEY,
) -> Option<String> {
    let mut prop_type: DEVPROPTYPE = 0;
    let mut required = 0u32;
    unsafe {
        let ok = SetupDiGetDevicePropertyW(
            hdev,
            devinfo,
            key,
            &mut prop_type,
            std::ptr::null_mut(),
            0,
            &mut required,
            0,
        );
        if ok == 0 {
            let code = GetLastError();
            if code != ERROR_INSUFFICIENT_BUFFER || required == 0 {
                return None;
            }
        }
        let mut buf = vec![0u8; required as usize];
        let ok2 = SetupDiGetDevicePropertyW(
            hdev,
            devinfo,
            key,
            &mut prop_type,
            buf.as_mut_ptr(),
            required,
            &mut required,
            0,
        );
        if ok2 == 0 {
            return None;
        }
        if prop_type != DEVPROP_TYPE_STRING {
            return None;
        }
        parse_reg_sz(&buf)
    }
}

fn reg_query_port_name(hkey: HKEY) -> Option<String> {
    let name = to_wide_null("PortName");
    let mut typ = 0u32;
    let mut size = 0u32;
    unsafe {
        let r = RegQueryValueExW(
            hkey,
            name.as_ptr(),
            std::ptr::null(),
            &mut typ,
            std::ptr::null_mut(),
            &mut size,
        );
        if r != ERROR_SUCCESS || size < 2 {
            return None;
        }
    }
    let mut buf = vec![0u8; size as usize];
    unsafe {
        let r2 = RegQueryValueExW(
            hkey,
            name.as_ptr(),
            std::ptr::null(),
            &mut typ,
            buf.as_mut_ptr(),
            &mut size,
        );
        if r2 != ERROR_SUCCESS || typ != REG_SZ {
            return None;
        }
    }
    parse_reg_sz(&buf)
}

fn is_slime_smol_serial_hardware(ids: &[String]) -> bool {
    ids.iter().any(|line| {
        let u = line.to_uppercase();
        u.contains(SLIME_SMOL_VID)
            && (u.contains(SLIME_SMOL_TRACKER_PID) || u.contains(SLIME_SMOL_RECEIVER_PID))
    })
}

fn best_display_for_port_device(
    hdev: HDEVINFO,
    devinfo: &mut SP_DEVINFO_DATA,
) -> Option<String> {
    let hardware_ids = get_registry_property_multi_sz(hdev, devinfo, SPDRP_HARDWAREID)?;
    if !is_slime_smol_serial_hardware(&hardware_ids) {
        return None;
    }

    let hkey = unsafe {
        SetupDiOpenDevRegKey(
            hdev,
            devinfo,
            DICS_FLAG_GLOBAL,
            0,
            DIREG_DEV,
            KEY_READ,
        )
    };
    if hkey == INVALID_HANDLE_VALUE {
        return None;
    }
    let port_name = reg_query_port_name(hkey);
    unsafe {
        let _ = RegCloseKey(hkey);
    }
    let port_name = port_name?;

    let bus_reported =
        get_device_property_string(hdev, devinfo, &DEVPKEY_Device_BusReportedDeviceDesc)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

    let friendly = get_registry_property_sz(hdev, devinfo, SPDRP_FRIENDLYNAME)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let label = bus_reported.or(friendly)?;
    Some(format!("{}|{}", port_name.to_uppercase(), label))
}

/// Returns map: uppercase `COMn` -> display suffix (bus reported or friendly name text only).
pub fn slime_smol_port_bus_reported_map() -> HashMap<String, String> {
    let mut map = HashMap::new();
    let hdev: HDEVINFO = unsafe {
        SetupDiGetClassDevsW(
            &GUID_DEVCLASS_PORTS,
            std::ptr::null(),
            std::ptr::null_mut(),
            DIGCF_PRESENT,
        )
    };
    if hdev == -1isize {
        return map;
    }

    let mut index = 0u32;
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

        if let Some(combined) = best_display_for_port_device(hdev, &mut devinfo) {
            let mut parts = combined.splitn(2, '|');
            let com_key = parts.next().unwrap_or("").to_string();
            let label = parts.next().unwrap_or("").to_string();
            if !com_key.is_empty() && !label.is_empty() {
                map.insert(com_key, label);
            }
        }
    }
    unsafe {
        SetupDiDestroyDeviceInfoList(hdev);
    }
    map
}
