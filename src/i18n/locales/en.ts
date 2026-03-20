export default {
  common: {
    refresh: "Refresh",
    connect: "Connect",
    disconnect: "Disconnect",
    clear: "Clear",
    on: "ON",
    off: "OFF",
    success: "Success",
    error: "Error",
    info: "Info",
    unknown_error: "Unknown Error",
    processing: "Processing...",
    execution_time: "Elapsed: {elapsed}s / Estimated: {estimated}s"
  },
  app: {
    title: "FoxDock Console",
    subtitle: "FoxSnack Tracker Base",
    debug_btn: "Debug",
    info_btn: "Info",
    info_title: "Version Info",
    app_name: "App Name",
    app_version: "App Version",
    protocol_version: "Protocol Version",
    close_btn: "Close"
  },
  connection: {
    title: "Connection Management",
    select_port: "Please select dock port",
    current_status: "Current Status:",
    connected: "Connected",
    disconnected: "Disconnected",
    project: "Project:",
    version: "Version:",
    mcu: "MCU:"
  },
  tracker_status: {
    title: "Tracker Status",
    slot: "Slot {id}",
    inserted: "Inserted",
    not_inserted: "Not Inserted"
  },
  tracker_control: {
    title: "Tracker Control",
    single_section: "Single Tracker Control",
    all_section: "All Trackers Control",
    dock_section: "Dock Settings",
    slot_label: "Slot",
    led_label: "Dock LED:",
    auto_sleep_label: "Auto Sleep On Insert:",
    refresh_status: "Refresh Status",
    bl_mode_label: "BL Boot Mode",
    set_bl_mode: "Set BL Mode",
    current_bl_mode: "Current Mode: {mode} ({name})",
    bl_mode_option_0: "Double RST",
    bl_mode_option_1: "RST + 4x CSW",
    bl_mode_option_2: "RST + 8x CSW",
    actions: {
      ret: "Reset",
      bl: "Bootloader",
      wake_up: "Wake Up",
      sleep: "Sleep",
      pair: "Pair",
      ret_all: "Reset All",
      bl_all: "Bootloader All",
      wake_up_all: "Wake Up All",
      sleep_all: "Sleep All",
      pair_all: "Pair All"
    }
  },
  nav: {
    home: "Home",
    flashing: "Flashing",
    settings: "Settings"
  },
  flashing: {
    title: "Tracker Flashing",
    placeholder: "Flashing feature is under development..."
  },
  settings: {
    title: "Settings",
    placeholder: "Settings feature is under development..."
  },
  notifications: {
    scan_found: "Scanned {count} available docks",
    scan_failed: "Scan failed: {msg}",
    status_read_failed: "Failed to read connection status: {msg}",
    select_port_first: "Please select a dock port first",
    already_connected: "Already connected to dock {name}. Disconnect first.",
    connect_success: "Connected to {name}",
    connect_failed: "Connection failed: {msg}",
    disconnect_success: "Disconnected",
    disconnect_failed: "Disconnection failed: {msg}",
    usb_disconnected: "Dock USB disconnected. Connection state has been reset.",
    info_read_failed: "Failed to read dock info: {msg}",
    tracker_status_failed: "Failed to read tracker status: {msg}",
    bl_mode_read_failed: "Failed to read BL boot mode: {msg}",
    bl_mode_set_success: "BL boot mode set to {mode} ({name})",
    bl_mode_set_failed: "Failed to set BL boot mode: {msg}",
    auto_sleep_read_failed: "Failed to read auto sleep status: {msg}",
    auto_sleep_set_success: "Auto sleep on insert set to {status}",
    auto_sleep_set_failed: "Failed to set auto sleep on insert: {msg}",
    action_success: "Action success: {cmd} #{id}",
    action_all_success: "Action success: {cmd}",
    action_failed: "Action failed: {msg}",
    led_success: "Dock LED turned {status}",
    led_failed: "LED control failed: {msg}",
    version_read_failed: "Failed to read version info: {msg}",
    event_inserted: "[Event] Slot #{id} Inserted",
    event_removed: "[Event] Slot #{id} Removed",
    event_boot: "[Event] Dock rebooted: {project} v{version}"
  },
  backend_errors: {
    dock_not_connected: "Dock is not connected",
    command_interrupted: "Command was interrupted",
    device_timeout: "Device response timeout",
    command_overridden: "Overridden by a newer command",
    unexpected_response: "Device did not return {expected} response. Payload: {value}",
    setupapi_failed: "SetupDiGetClassDevsW failed",
    port_not_found: "Target dock port not found. Please refresh the device list first",
    open_serial_failed: "Failed to open serial port: {error}",
    initial_status_failed: "Connected but failed to fetch initial status: {detail}",
    tracker_id_out_of_range: "Tracker ID must be between 1 and 10",
    unsupported_single_action: "Unsupported single tracker action",
    unsupported_all_action: "Unsupported all-trackers action"
  }
};
