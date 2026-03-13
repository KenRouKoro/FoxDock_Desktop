export default {
  common: {
    refresh: "刷新",
    connect: "连接",
    disconnect: "断开",
    clear: "清除",
    on: "开",
    off: "关",
    success: "成功",
    error: "失败",
    info: "提示",
    unknown_error: "未知错误",
    processing: "正在处理中...",
    execution_time: "已执行: {elapsed}s / 预计: {estimated}s"
  },
  app: {
    title: "FoxDock 桌面控制台",
    subtitle: "10 槽位追踪器底座管理",
    debug_btn: "调试"
  },
  connection: {
    title: "连接管理",
    select_port: "请选择底座端口",
    current_status: "当前连接：",
    connected: "已连接",
    disconnected: "未连接",
    project: "项目：",
    version: "版本：",
    mcu: "MCU："
  },
  tracker_status: {
    title: "追踪器状态",
    slot: "槽位 {id}",
    inserted: "已插入",
    not_inserted: "未插入"
  },
  tracker_control: {
    title: "追踪器控制",
    slot_label: "槽位",
    led_label: "底座 LED：",
    refresh_status: "刷新状态",
    actions: {
      ret: "复位",
      bl: "Bootloader",
      sleep: "休眠",
      pair: "配对",
      ret_all: "全部复位",
      bl_all: "全部 Bootloader",
      sleep_all: "全部休眠",
      pair_all: "全部配对"
    }
  },
  notifications: {
    scan_found: "扫描到 {count} 个可用底座",
    scan_failed: "扫描失败：{msg}",
    status_read_failed: "读取连接状态失败：{msg}",
    select_port_first: "请先选择底座端口",
    already_connected: "已连接到底座 {name}，请先断开后再连接",
    connect_success: "已连接 {name}",
    connect_failed: "连接失败：{msg}",
    disconnect_success: "已断开连接",
    disconnect_failed: "断开失败：{msg}",
    usb_disconnected: "检测到底座 USB 已断开，连接状态已重置",
    info_read_failed: "读取底座信息失败：{msg}",
    tracker_status_failed: "读取追踪器状态失败：{msg}",
    action_success: "执行成功：{cmd} #{id}",
    action_all_success: "执行成功：{cmd}",
    action_failed: "执行失败：{msg}",
    led_success: "底座 LED 已{status}",
    led_failed: "LED 控制失败：{msg}",
    event_inserted: "[事件] 槽位 #{id} 已插入",
    event_removed: "[事件] 槽位 #{id} 已拔出",
    event_boot: "[事件] 底座已重启: {project} v{version}"
  }
};
