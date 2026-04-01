/** 底座串口枚举项 */
export type DockPort = {
  portName: string;
  displayName: string;
  serialNumber: string | null;
};

/** 底座 info 指令解析结果 */
export type DockInfo = {
  project: string;
  version: string;
  mcu: string;
  extra?: Record<string, unknown>;
};

/** 单路追踪器插入与 USB 拓扑状态 */
export type TrackerStatus = {
  id: number;
  inserted: boolean;
  usbPath?: string;
};
