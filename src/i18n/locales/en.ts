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
    subtitle: "10-Slot Tracker Dock Manager",
    debug_btn: "Debug"
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
    slot_label: "Slot",
    led_label: "Dock LED:",
    refresh_status: "Refresh Status",
    actions: {
      ret: "Reset",
      bl: "Bootloader",
      sleep: "Sleep",
      pair: "Pair",
      ret_all: "Reset All",
      bl_all: "Bootloader All",
      sleep_all: "Sleep All",
      pair_all: "Pair All"
    }
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
    action_success: "Action success: {cmd} #{id}",
    action_all_success: "Action success: {cmd}",
    action_failed: "Action failed: {msg}",
    led_success: "Dock LED turned {status}",
    led_failed: "LED control failed: {msg}",
    event_inserted: "[Event] Slot #{id} Inserted",
    event_removed: "[Event] Slot #{id} Removed",
    event_boot: "[Event] Dock rebooted: {project} v{version}"
  }
};
