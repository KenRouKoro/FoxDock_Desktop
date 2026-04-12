import type {
  SerialConsoleBuildResult,
  SerialConsoleField,
  SerialConsoleLocalCommand,
  SerialConsoleRemoteCommand,
  SerialConsoleDeviceType,
} from "../types/serialConsole";

const HEX_12 = /^[0-9a-f]{12}$/;
const HEX_16 = /^[0-9a-f]{16}$/;

function parseOptionalNumber(raw: string | number | boolean): number | null {
  if (raw === "" || raw === false) return null;
  if (typeof raw === "boolean") return null;
  if (typeof raw === "number") return Number.isFinite(raw) ? raw : null;
  const t = String(raw).trim();
  if (!t) return null;
  const n = Number(t);
  return Number.isFinite(n) ? n : null;
}

function parseRequiredNumber(
  raw: string | number | boolean,
  integer?: boolean,
): { ok: true; n: number } | { ok: false; messageKey: string } {
  const n = parseOptionalNumber(raw);
  if (n === null) {
    return { ok: false, messageKey: "serial_console.validation.number_required" };
  }
  if (integer && !Number.isInteger(n)) {
    return { ok: false, messageKey: "serial_console.validation.integer_required" };
  }
  return { ok: true, n };
}

export function initParamFormValues(
  fields: SerialConsoleField[],
): Record<string, string | number | boolean> {
  const o: Record<string, string | number | boolean> = {};
  for (const f of fields) {
    if (f.kind === "text") {
      o[f.id] = f.defaultValue ?? "";
    } else if (f.kind === "number") {
      o[f.id] = f.defaultValue ?? "";
    } else if (f.kind === "select") {
      o[f.id] = f.defaultValue ?? f.options[0]?.value ?? "";
    } else {
      o[f.id] = f.defaultOn ?? false;
    }
  }
  return o;
}

export function tryBuildLocalLine(
  cmd: SerialConsoleLocalCommand,
  values: Record<string, string | number | boolean>,
): SerialConsoleBuildResult {
  if (!cmd.parametric) {
    return cmd.command
      ? { ok: true, text: cmd.command }
      : { ok: false, messageKey: "serial_console.validation.no_command" };
  }
  return cmd.parametric.buildLine(values);
}

export function tryBuildRemoteTail(
  cmd: SerialConsoleRemoteCommand,
  values: Record<string, string | number | boolean>,
): SerialConsoleBuildResult {
  if (!cmd.parametric) {
    return cmd.remoteTail
      ? { ok: true, text: cmd.remoteTail }
      : { ok: false, messageKey: "serial_console.validation.no_command" };
  }
  return cmd.parametric.buildTail(values);
}

export function remoteCommandUsesOnlyAll(cmd: SerialConsoleRemoteCommand): boolean {
  if (cmd.remoteOnlyAll) return true;
  return Boolean(cmd.parametric?.remoteOnlyAll);
}

const trackerCommands: SerialConsoleLocalCommand[] = [
  { key: "help", command: "help" },
  { key: "info", command: "info" },
  { key: "uptime", command: "uptime" },
  { key: "battery", command: "battery" },
  { key: "scan", command: "scan" },
  { key: "calibrate", command: "calibrate" },
  { key: "6side", command: "6-side" },
  { key: "mag_on", command: "mag on" },
  { key: "mag_off", command: "mag off" },
  { key: "mag_clear", command: "mag clear" },
  { key: "mag_cal", command: "mag cal" },
  { key: "sens_reset", command: "sens reset" },
  {
    key: "sens_set",
    parametric: {
      fields: [
        {
          id: "x",
          kind: "number",
          labelKey: "serial_console.param.sens_axis_x",
          required: true,
        },
        {
          id: "y",
          kind: "number",
          labelKey: "serial_console.param.sens_axis_y",
          required: true,
        },
        {
          id: "z",
          kind: "number",
          labelKey: "serial_console.param.sens_axis_z",
          required: true,
        },
      ],
      buildLine: (v) => {
        const xr = parseRequiredNumber(v.x);
        const yr = parseRequiredNumber(v.y);
        const zr = parseRequiredNumber(v.z);
        if (!xr.ok) return xr;
        if (!yr.ok) return yr;
        if (!zr.ok) return zr;
        return { ok: true, text: `sens ${xr.n},${yr.n},${zr.n}` };
      },
    },
  },
  { key: "tcal_status", command: "tcal status" },
  { key: "tcal_clear", command: "tcal clear" },
  { key: "tcal_dump", command: "tcal dump" },
  { key: "tcal_check", command: "tcal check" },
  {
    key: "tcal_test",
    parametric: {
      fields: [
        {
          id: "temp",
          kind: "text",
          labelKey: "serial_console.param.tcal_test_temp",
          placeholderKey: "serial_console.param.tcal_test_temp_ph",
          required: false,
          defaultValue: "",
        },
      ],
      buildLine: (v) => {
        const t = String(v.temp ?? "").trim();
        if (!t) return { ok: true, text: "tcal test" };
        const n = Number(t);
        if (!Number.isFinite(n)) {
          return { ok: false, messageKey: "serial_console.validation.number_required" };
        }
        return { ok: true, text: `tcal test ${n}` };
      },
    },
  },
  {
    key: "tcal_remove",
    parametric: {
      fields: [
        {
          id: "index",
          kind: "number",
          labelKey: "serial_console.param.tcal_index",
          min: 0,
          max: 9999,
          integer: true,
          required: true,
          defaultValue: 0,
        },
      ],
      buildLine: (v) => {
        const ir = parseRequiredNumber(v.index, true);
        if (!ir.ok) return ir;
        return { ok: true, text: `tcal remove ${ir.n}` };
      },
    },
  },
  { key: "tcal_auto_on", command: "tcal auto on" },
  { key: "tcal_auto_off", command: "tcal auto off" },
  { key: "tcal_boot_on", command: "tcal boot on" },
  { key: "tcal_boot_off", command: "tcal boot off" },
  { key: "tdma_on", command: "tdma on" },
  { key: "tdma_off", command: "tdma off" },
  {
    key: "set_address",
    parametric: {
      fields: [
        {
          id: "address",
          kind: "text",
          labelKey: "serial_console.param.set_address",
          placeholderKey: "serial_console.param.set_address_ph",
          required: true,
          defaultValue: "",
        },
      ],
      buildLine: (v) => {
        const addr = String(v.address ?? "").trim().toLowerCase();
        if (!HEX_16.test(addr)) {
          return { ok: false, messageKey: "serial_console.validation.hex16_nonzero" };
        }
        if (/^0+$/.test(addr)) {
          return { ok: false, messageKey: "serial_console.validation.address_not_zero" };
        }
        return { ok: true, text: `set ${addr}` };
      },
    },
  },
  {
    key: "channel",
    parametric: {
      fields: [
        {
          id: "ch",
          kind: "number",
          labelKey: "serial_console.param.channel",
          min: 0,
          max: 100,
          integer: true,
          required: true,
          defaultValue: 84,
        },
      ],
      buildLine: (v) => {
        const r = parseRequiredNumber(v.ch, true);
        if (!r.ok) return r;
        if (r.n < 0 || r.n > 100) {
          return { ok: false, messageKey: "serial_console.validation.channel_tracker_range" };
        }
        return { ok: true, text: `channel ${r.n}` };
      },
    },
  },
  { key: "pair", command: "pair" },
  { key: "clear", command: "clear" },
  { key: "clearchannel", command: "clearchannel" },
  { key: "reboot", command: "reboot" },
  { key: "dfu", command: "dfu" },
  { key: "dfu_ota", command: "dfu ota" },
  {
    key: "debug",
    parametric: {
      fields: [
        {
          id: "duration",
          kind: "number",
          labelKey: "serial_console.param.debug_duration",
          min: 1,
          max: 60,
          integer: true,
          required: true,
          defaultValue: 1,
        },
      ],
      buildLine: (v) => {
        const r = parseRequiredNumber(v.duration, true);
        if (!r.ok) return r;
        if (r.n < 1 || r.n > 60) {
          return { ok: false, messageKey: "serial_console.validation.debug_duration" };
        }
        return { ok: true, text: `debug ${r.n}` };
      },
    },
  },
  { key: "ping", command: "ping" },
  { key: "meow", command: "meow" },
  { key: "reset_zro", command: "reset zro" },
  { key: "reset_acc", command: "reset acc" },
  { key: "reset_mag", command: "reset mag" },
  { key: "reset_sens", command: "reset sens" },
  { key: "reset_tcal", command: "reset tcal" },
  { key: "reset_bat", command: "reset bat" },
  { key: "reset_fusion", command: "reset fusion" },
  { key: "reset_all", command: "reset all" },
];

const receiverCommands: SerialConsoleLocalCommand[] = [
  { key: "help", command: "help" },
  { key: "info", command: "info" },
  { key: "uptime", command: "uptime" },
  { key: "list", command: "list" },
  {
    key: "add",
    parametric: {
      fields: [
        {
          id: "address",
          kind: "text",
          labelKey: "serial_console.param.add_address",
          placeholderKey: "serial_console.param.add_address_ph",
          required: true,
          defaultValue: "",
        },
      ],
      buildLine: (v) => {
        const addr = String(v.address ?? "").trim().toLowerCase();
        if (!HEX_12.test(addr)) {
          return { ok: false, messageKey: "serial_console.validation.hex12" };
        }
        return { ok: true, text: `add ${addr}` };
      },
    },
  },
  {
    key: "pair",
    parametric: {
      fields: [
        {
          id: "mode",
          kind: "select",
          labelKey: "serial_console.param.pair_mode",
          defaultValue: "default",
          options: [
            { value: "default", labelKey: "serial_console.pair_mode_default" },
            { value: "zero", labelKey: "serial_console.pair_mode_zero" },
            { value: "count", labelKey: "serial_console.pair_mode_count" },
          ],
        },
        {
          id: "count",
          kind: "number",
          labelKey: "serial_console.param.pair_count",
          min: 1,
          max: 255,
          integer: true,
          required: true,
          defaultValue: 1,
        },
      ],
      buildLine: (v) => {
        const mode = String(v.mode ?? "default");
        if (mode === "default") return { ok: true, text: "pair" };
        if (mode === "zero") return { ok: true, text: "pair 0" };
        const r = parseRequiredNumber(v.count, true);
        if (!r.ok) return r;
        if (r.n < 1) {
          return { ok: false, messageKey: "serial_console.validation.pair_count_min" };
        }
        return { ok: true, text: `pair ${r.n}` };
      },
    },
  },
  { key: "exit", command: "exit" },
  { key: "clear", command: "clear" },
  {
    key: "stats",
    parametric: {
      fields: [
        {
          id: "mode",
          kind: "select",
          labelKey: "serial_console.param.stats_mode",
          defaultValue: "toggle",
          options: [
            { value: "toggle", labelKey: "serial_console.stats_mode_toggle" },
            { value: "timed", labelKey: "serial_console.stats_mode_timed" },
          ],
        },
        {
          id: "seconds",
          kind: "number",
          labelKey: "serial_console.param.stats_seconds",
          min: 0,
          max: 86400,
          integer: true,
          required: true,
          defaultValue: 60,
        },
      ],
      buildLine: (v) => {
        const mode = String(v.mode ?? "toggle");
        if (mode === "toggle") return { ok: true, text: "stats" };
        const r = parseRequiredNumber(v.seconds, true);
        if (!r.ok) return r;
        if (r.n < 0 || r.n > 86400) {
          return { ok: false, messageKey: "serial_console.validation.stats_seconds" };
        }
        return { ok: true, text: `stats ${r.n}` };
      },
    },
  },
  { key: "resetstats", command: "resetstats" },
  { key: "rssi_scan", command: "rssi_scan" },
  {
    key: "channel",
    parametric: {
      fields: [
        {
          id: "ch",
          kind: "number",
          labelKey: "serial_console.param.channel",
          min: 1,
          max: 100,
          integer: true,
          required: true,
          defaultValue: 84,
        },
      ],
      buildLine: (v) => {
        const r = parseRequiredNumber(v.ch, true);
        if (!r.ok) return r;
        if (r.n < 1 || r.n > 100) {
          return { ok: false, messageKey: "serial_console.validation.channel_receiver_range" };
        }
        return { ok: true, text: `channel ${r.n}` };
      },
    },
  },
  { key: "clearchannel", command: "clearchannel" },
  { key: "reboot", command: "reboot" },
  { key: "dfu", command: "dfu" },
  { key: "meow", command: "meow" },
];

const receiverRemoteCommands: SerialConsoleRemoteCommand[] = [
  { key: "remote_shutdown", remoteTail: "shutdown" },
  { key: "remote_calibrate", remoteTail: "calibrate" },
  { key: "remote_6side", remoteTail: "6-side" },
  { key: "remote_reboot", remoteTail: "reboot" },
  { key: "remote_dfu", remoteTail: "dfu" },
  { key: "remote_scan", remoteTail: "scan" },
  { key: "remote_ping", remoteTail: "ping" },
  { key: "remote_meow", remoteTail: "meow" },
  { key: "remote_clear", remoteTail: "clear" },
  { key: "remote_fusion", remoteTail: "fusion" },
  {
    key: "remote_mag",
    parametric: {
      remoteOnlyAll: false,
      fields: [
        {
          id: "action",
          kind: "select",
          labelKey: "serial_console.param.remote_mag_action",
          defaultValue: "on",
          options: [
            { value: "on", labelKey: "serial_console.remote_mag_opt_on" },
            { value: "off", labelKey: "serial_console.remote_mag_opt_off" },
            { value: "clear", labelKey: "serial_console.remote_mag_opt_clear" },
            { value: "cal", labelKey: "serial_console.remote_mag_opt_cal" },
            { value: "calibrate", labelKey: "serial_console.remote_mag_opt_calibrate" },
          ],
        },
      ],
      buildTail: (v) => {
        const a = String(v.action ?? "on");
        const allowed = new Set(["on", "off", "clear", "cal", "calibrate"]);
        if (!allowed.has(a)) {
          return { ok: false, messageKey: "serial_console.validation.invalid_select" };
        }
        return { ok: true, text: `mag ${a}` };
      },
    },
  },
  {
    key: "remote_channel",
    parametric: {
      remoteOnlyAll: true,
      fields: [
        {
          id: "ch",
          kind: "number",
          labelKey: "serial_console.param.channel",
          min: 1,
          max: 100,
          integer: true,
          required: true,
          defaultValue: 84,
        },
      ],
      buildTail: (v) => {
        const r = parseRequiredNumber(v.ch, true);
        if (!r.ok) return r;
        if (r.n < 1 || r.n > 100) {
          return { ok: false, messageKey: "serial_console.validation.channel_receiver_range" };
        }
        return { ok: true, text: `channel ${r.n}` };
      },
    },
  },
  { key: "remote_clearchannel", remoteTail: "clearchannel", remoteOnlyAll: true },
  {
    key: "remote_sens_set",
    parametric: {
      fields: [
        {
          id: "x",
          kind: "number",
          labelKey: "serial_console.param.sens_axis_x",
          required: true,
        },
        {
          id: "y",
          kind: "number",
          labelKey: "serial_console.param.sens_axis_y",
          required: true,
        },
        {
          id: "z",
          kind: "number",
          labelKey: "serial_console.param.sens_axis_z",
          required: true,
        },
      ],
      buildTail: (v) => {
        const xr = parseRequiredNumber(v.x);
        const yr = parseRequiredNumber(v.y);
        const zr = parseRequiredNumber(v.z);
        if (!xr.ok) return xr;
        if (!yr.ok) return yr;
        if (!zr.ok) return zr;
        return { ok: true, text: `sens ${xr.n},${yr.n},${zr.n}` };
      },
    },
  },
  { key: "remote_sens_reset", remoteTail: "sens reset" },
  { key: "remote_tcal_on", remoteTail: "tcal on" },
  { key: "remote_tcal_off", remoteTail: "tcal off" },
  { key: "remote_tcal_auto_on", remoteTail: "tcal auto on" },
  { key: "remote_tcal_auto_off", remoteTail: "tcal auto off" },
  { key: "remote_tcal_boot_on", remoteTail: "tcal boot on" },
  { key: "remote_tcal_boot_off", remoteTail: "tcal boot off" },
  { key: "remote_tcal_clear", remoteTail: "tcal clear" },
  { key: "remote_tdma_on", remoteTail: "tdma on" },
  { key: "remote_tdma_off", remoteTail: "tdma off" },
  { key: "remote_test_on", remoteTail: "test on" },
  { key: "remote_test_off", remoteTail: "test off" },
  {
    key: "remote_reset",
    parametric: {
      fields: [
        {
          id: "kind",
          kind: "select",
          labelKey: "serial_console.param.remote_reset_kind",
          defaultValue: "zro",
          options: [
            { value: "zro", labelKey: "serial_console.remote_reset_opt_zro" },
            { value: "acc", labelKey: "serial_console.remote_reset_opt_acc" },
            { value: "bat", labelKey: "serial_console.remote_reset_opt_bat" },
            { value: "mag", labelKey: "serial_console.remote_reset_opt_mag" },
            { value: "tcal", labelKey: "serial_console.remote_reset_opt_tcal" },
            { value: "fusion", labelKey: "serial_console.remote_reset_opt_fusion" },
          ],
        },
      ],
      buildTail: (v) => {
        const k = String(v.kind ?? "zro");
        const allowed = new Set(["zro", "acc", "bat", "mag", "tcal", "fusion"]);
        if (!allowed.has(k)) {
          return { ok: false, messageKey: "serial_console.validation.invalid_select" };
        }
        return { ok: true, text: `reset ${k}` };
      },
    },
  },
];

export function getSerialConsoleCommands(
  deviceType: SerialConsoleDeviceType,
): SerialConsoleLocalCommand[] {
  return deviceType === "receiver" ? receiverCommands : trackerCommands;
}

export function getReceiverRemoteCommands(): SerialConsoleRemoteCommand[] {
  return receiverRemoteCommands;
}

function normalizeSentLine(text: string): string {
  return text.replace(/\r\n/g, "\n").trim();
}

/**
 * True if the sent line is DFU / DFU OTA (device typically disconnects and does not return as the same serial).
 * Local: `dfu`, `dfu ota`; remote: `send <target> dfu`.
 */
export function isDfuCommand(text: string): boolean {
  const line = normalizeSentLine(text);
  if (!line) return false;
  const lower = line.toLowerCase();
  if (lower === "dfu" || lower === "dfu ota") return true;
  return /^send\s+\S+\s+dfu\s*$/i.test(line);
}

/** 接收器右键「指令列表」两级菜单分组（labelKey 在 i18n `tracker_status.*`） */
export const RECEIVER_CONTEXT_MENU_GROUPS: { labelKey: string; keys: string[] }[] = [
  {
    labelKey: "tracker_status.receiver_menu_general",
    keys: ["help", "info", "uptime", "list", "clear", "exit"],
  },
  {
    labelKey: "tracker_status.receiver_menu_rf",
    keys: ["add", "pair", "channel", "clearchannel", "rssi_scan", "stats", "resetstats"],
  },
  {
    labelKey: "tracker_status.receiver_menu_system",
    keys: ["reboot", "dfu", "meow"],
  },
];

export function buildReceiverContextMenuGroups(): {
  labelKey: string;
  commands: SerialConsoleLocalCommand[];
}[] {
  const cmds = getSerialConsoleCommands("receiver");
  const byKey = new Map(cmds.map((c) => [c.key, c]));
  return RECEIVER_CONTEXT_MENU_GROUPS.map((g) => ({
    labelKey: g.labelKey,
    commands: g.keys
      .map((k) => byKey.get(k))
      .filter((c): c is SerialConsoleLocalCommand => Boolean(c)),
  }));
}

export function findReceiverLocalCommandByKey(key: string): SerialConsoleLocalCommand | null {
  for (const g of buildReceiverContextMenuGroups()) {
    for (const c of g.commands) {
      if (c.key === key) return c;
    }
  }
  return null;
}
