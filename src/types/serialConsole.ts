export type SerialConsoleDeviceType = "tracker" | "receiver";

export type SerialConsoleTargetHint = {
  deviceType: SerialConsoleDeviceType;
  trackerId?: number | null;
};

export type SerialConsoleState = {
  connected: boolean;
  portName?: string | null;
  displayName?: string | null;
  deviceType: SerialConsoleDeviceType;
  trackerId?: number | null;
};

export type ReceiverStatus = {
  inserted: boolean;
  portName?: string | null;
  displayName?: string | null;
};

export type SerialConsoleLog = {
  direction: string;
  content: string;
  timestamp: string;
};

export type SerialConsoleCommand = {
  key: string;
  command: string;
  descriptionKey?: string;
};
