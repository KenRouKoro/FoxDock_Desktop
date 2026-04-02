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

/** 参数表单构建结果 */
export type SerialConsoleBuildResult =
  | { ok: true; text: string }
  | {
      ok: false;
      messageKey: string;
      messageParams?: Record<string, string | number>;
    };

export type SerialConsoleFieldOption = {
  value: string;
  labelKey: string;
};

export type SerialConsoleField =
  | {
      id: string;
      kind: "text";
      labelKey: string;
      placeholderKey?: string;
      required?: boolean;
      defaultValue?: string;
    }
  | {
      id: string;
      kind: "number";
      labelKey: string;
      min?: number;
      max?: number;
      integer?: boolean;
      required?: boolean;
      defaultValue?: number;
    }
  | {
      id: string;
      kind: "select";
      labelKey: string;
      options: SerialConsoleFieldOption[];
      defaultValue?: string;
    }
  | {
      id: string;
      kind: "toggle";
      labelKey: string;
      onValue: string;
      offValue: string;
      defaultOn?: boolean;
    };

export type SerialConsoleParametricLocal = {
  fields: SerialConsoleField[];
  buildLine: (
    values: Record<string, string | number | boolean>,
  ) => SerialConsoleBuildResult;
};

export type SerialConsoleParametricRemote = {
  fields: SerialConsoleField[];
  buildTail: (
    values: Record<string, string | number | boolean>,
  ) => SerialConsoleBuildResult;
  /** 发送时目标强制为 all（如 channel / clearchannel） */
  remoteOnlyAll?: boolean;
};

/** 本机快捷指令（追踪器 / 接收器） */
export type SerialConsoleLocalCommand = {
  key: string;
  /** 无参：整行发送 */
  command?: string;
  parametric?: SerialConsoleParametricLocal;
  descriptionKey?: string;
};

/** 接收器远程 send 指令 */
export type SerialConsoleRemoteCommand = {
  key: string;
  /** 无参：send <target> 后的尾部 */
  remoteTail?: string;
  /** 无参指令也仅允许 all 时使用 */
  remoteOnlyAll?: boolean;
  parametric?: SerialConsoleParametricRemote;
  descriptionKey?: string;
};
