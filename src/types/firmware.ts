export type FirmwarePhase =
  | "idle"
  | "ready"
  | "entering_bl"
  | "waiting_bootloader"
  | "copying"
  | "verifying"
  | "success"
  | "error";

export type FirmwareMode = "manual" | "auto_slot" | "batch_all";

export type FirmwareRunState =
  | "idle"
  | "waiting"
  | "queued"
  | "running"
  | "success"
  | "warning"
  | "skipped"
  | "error";

export type FirmwareRunItem = {
  state: FirmwareRunState;
  message: string;
};

export type FirmwareFile = {
  name: string;
  size: number;
  bytes: Uint8Array;
};

export type FirmwareProgressEvent = {
  trackerId: number;
  phase: FirmwarePhase;
  progress: number;
  message: string;
};

export type FirmwareFlashResult = {
  trackerId: number;
  success: boolean;
  warning: boolean;
  phase: FirmwarePhase;
  progress: number;
  message: string;
  fileName: string;
  drivePath?: string;
};

/** 固件页槽位队列展示项 */
export type FirmwareSlotStatus = {
  id: number;
  inserted: boolean;
  usbPath: string;
  state: FirmwareRunState;
  message: string;
};

export type BleOtaPhase =
  | "idle"
  | "scanning"
  | "connecting"
  | "starting"
  | "init"
  | "transferring"
  | "validating"
  | "activating"
  | "success"
  | "error";

export type BleDfuDevice = {
  peripheralId: string;
  name: string;
  address: string;
  rssi: number | null;
};

export type BleOtaPackageInfo = {
  firmwareType: string;
  binFileName: string;
  datFileName: string;
  binSize: number;
  datSize: number;
};

export type BleOtaProgressEvent = {
  phase: BleOtaPhase | string;
  progress: number;
  message: string;
  bytesTransferred?: number;
  totalBytes?: number;
};
