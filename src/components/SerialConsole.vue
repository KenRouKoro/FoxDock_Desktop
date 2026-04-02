<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import BaseButton from "./ui/BaseButton.vue";
import BasePanel from "./ui/BasePanel.vue";
import BaseSelect from "./ui/BaseSelect.vue";
import type { DockPort } from "../types/dock";
import type {
  SerialConsoleDeviceType,
  SerialConsoleField,
  SerialConsoleLocalCommand,
  SerialConsoleLog,
  SerialConsoleRemoteCommand,
  SerialConsoleState,
  SerialConsoleTargetHint,
} from "../types/serialConsole";
import {
  getSerialConsoleCommands,
  getReceiverRemoteCommands,
  initParamFormValues,
  remoteCommandUsesOnlyAll,
  tryBuildLocalLine,
  tryBuildRemoteTail,
} from "../utils/serialConsoleCommands";

const { t } = useI18n();
const LOG_FLUSH_DELAY_MS = 500;
const MAX_LOG_ENTRIES = 2000;

interface LogDisplaySegment {
  text: string;
  classes: string[];
}

interface ConsoleLogEntry extends SerialConsoleLog {
  segments: LogDisplaySegment[];
}

interface PendingConsoleLog {
  content: string;
  timestamp: string;
  timerId: number | null;
}

interface AnsiStyleState {
  fg: string | null;
  bg: string | null;
  bold: boolean;
  dim: boolean;
  italic: boolean;
  underline: boolean;
}

const searchParams = new URLSearchParams(window.location.search);

function parseDeviceType(raw: string | null): SerialConsoleDeviceType {
  return raw === "receiver" ? "receiver" : "tracker";
}

function parseTrackerId(raw: string | null): number | null {
  if (!raw) return null;
  const parsed = Number(raw);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : null;
}

function createAnsiStyleState(): AnsiStyleState {
  return {
    fg: null,
    bg: null,
    bold: false,
    dim: false,
    italic: false,
    underline: false,
  };
}

function cloneAnsiClasses(style: AnsiStyleState): string[] {
  const classes: string[] = [];
  if (style.bold) classes.push("ansi-bold");
  if (style.dim) classes.push("ansi-dim");
  if (style.italic) classes.push("ansi-italic");
  if (style.underline) classes.push("ansi-underline");
  if (style.fg) classes.push(`ansi-fg-${style.fg}`);
  if (style.bg) classes.push(`ansi-bg-${style.bg}`);
  return classes;
}

function pushAnsiText(segments: LogDisplaySegment[], text: string, style: AnsiStyleState): void {
  const cleaned = text.replace(/[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]/g, "");
  if (!cleaned) return;
  segments.push({
    text: cleaned,
    classes: cloneAnsiClasses(style),
  });
}

function applyAnsiCode(style: AnsiStyleState, code: number): void {
  const foregroundMap: Record<number, string> = {
    30: "black",
    31: "red",
    32: "green",
    33: "yellow",
    34: "blue",
    35: "magenta",
    36: "cyan",
    37: "white",
    90: "bright-black",
    91: "bright-red",
    92: "bright-green",
    93: "bright-yellow",
    94: "bright-blue",
    95: "bright-magenta",
    96: "bright-cyan",
    97: "bright-white",
  };
  const backgroundMap: Record<number, string> = {
    40: "black",
    41: "red",
    42: "green",
    43: "yellow",
    44: "blue",
    45: "magenta",
    46: "cyan",
    47: "white",
    100: "bright-black",
    101: "bright-red",
    102: "bright-green",
    103: "bright-yellow",
    104: "bright-blue",
    105: "bright-magenta",
    106: "bright-cyan",
    107: "bright-white",
  };

  if (code === 0) {
    style.fg = null;
    style.bg = null;
    style.bold = false;
    style.dim = false;
    style.italic = false;
    style.underline = false;
    return;
  }
  if (code === 1) {
    style.bold = true;
    return;
  }
  if (code === 2) {
    style.dim = true;
    return;
  }
  if (code === 3) {
    style.italic = true;
    return;
  }
  if (code === 4) {
    style.underline = true;
    return;
  }
  if (code === 22) {
    style.bold = false;
    style.dim = false;
    return;
  }
  if (code === 23) {
    style.italic = false;
    return;
  }
  if (code === 24) {
    style.underline = false;
    return;
  }
  if (code === 39) {
    style.fg = null;
    return;
  }
  if (code === 49) {
    style.bg = null;
    return;
  }
  if (foregroundMap[code]) {
    style.fg = foregroundMap[code];
    return;
  }
  if (backgroundMap[code]) {
    style.bg = backgroundMap[code];
  }
}

function parseAnsiSegments(content: string): LogDisplaySegment[] {
  const segments: LogDisplaySegment[] = [];
  const style = createAnsiStyleState();
  const ansiPattern = /\x1b\[([0-9;]*)([A-Za-z])/g;
  let cursor = 0;
  let match: RegExpExecArray | null = ansiPattern.exec(content);

  while (match) {
    const [token, rawCodes, command] = match;
    const tokenStart = match.index;
    if (tokenStart > cursor) {
      pushAnsiText(segments, content.slice(cursor, tokenStart), style);
    }
    if (command === "m") {
      const codes = rawCodes ? rawCodes.split(";").map((item) => Number(item || "0")) : [0];
      for (const code of codes) {
        if (Number.isFinite(code)) {
          applyAnsiCode(style, code);
        }
      }
    }
    cursor = tokenStart + token.length;
    match = ansiPattern.exec(content);
  }

  if (cursor < content.length) {
    pushAnsiText(
      segments,
      content
        .slice(cursor)
        .replace(/\x1b\[[0-9;?]*[ -/]*[@-~]/g, "")
        .replace(/\x1b/g, ""),
      style,
    );
  }

  return segments;
}

const targetDeviceType = ref<SerialConsoleDeviceType>(
  parseDeviceType(searchParams.get("deviceType")),
);
const targetTrackerId = ref<number | null>(parseTrackerId(searchParams.get("trackerId")));
const consoleState = ref<SerialConsoleState>({
  connected: false,
  deviceType: targetDeviceType.value,
  trackerId: targetTrackerId.value,
  portName: null,
  displayName: null,
});
const ports = ref<DockPort[]>([]);
const selectedPortName = ref("");
const inputText = ref("");
const appendNewline = ref(true);
const logs = ref<ConsoleLogEntry[]>([]);
const statusText = ref("");
const logContainer = ref<HTMLElement | null>(null);
const initError = ref("");
const pendingLogs = new Map<string, PendingConsoleLog>();

/** 远程 send：全部 / 指定 id（0–255） */
const remoteTargetChoice = ref<"all" | "id">("all");
const remoteTargetId = ref(0);

const paramModalOpen = ref(false);
const paramModalScope = ref<"local" | "remote">("local");
const paramModalCommand = ref<SerialConsoleLocalCommand | SerialConsoleRemoteCommand | null>(null);
const paramFormValues = ref<Record<string, string | number | boolean>>({});

let unlistenLog: (() => void) | null = null;
let unlistenState: (() => void) | null = null;
let unlistenTargetHint: (() => void) | null = null;

function resolveBackendMessage(raw: string): string {
  if (!raw.startsWith("i18n:")) return raw;
  const payload = raw.slice("i18n:".length);
  const separatorIndex = payload.indexOf("|");
  const key = separatorIndex === -1 ? payload : payload.slice(0, separatorIndex);
  if (!key) return raw;
  if (separatorIndex === -1) {
    return t(key);
  }
  try {
    return t(key, JSON.parse(payload.slice(separatorIndex + 1)));
  } catch {
    return t(key);
  }
}

function setStatus(message: string): void {
  statusText.value = resolveBackendMessage(message);
}

function applyState(nextState: SerialConsoleState): void {
  consoleState.value = nextState;
  if (nextState.connected) {
    targetDeviceType.value = nextState.deviceType;
    targetTrackerId.value = nextState.trackerId ?? null;
  }
  if (nextState.portName) {
    selectedPortName.value = nextState.portName;
  }
}

function syncScroll(): void {
  nextTick(() => {
    if (logContainer.value) {
      logContainer.value.scrollTop = logContainer.value.scrollHeight;
    }
  });
}

function clearPendingTimer(pending: PendingConsoleLog): void {
  if (pending.timerId !== null) {
    window.clearTimeout(pending.timerId);
    pending.timerId = null;
  }
}

function ensurePendingLog(direction: string): PendingConsoleLog {
  const existing = pendingLogs.get(direction);
  if (existing) {
    return existing;
  }
  const created: PendingConsoleLog = {
    content: "",
    timestamp: "",
    timerId: null,
  };
  pendingLogs.set(direction, created);
  return created;
}

function pushLogEntry(direction: string, content: string, timestamp: string): void {
  logs.value.push({
    direction,
    content,
    timestamp,
    segments: parseAnsiSegments(content),
  });
  if (logs.value.length > MAX_LOG_ENTRIES) {
    logs.value.splice(0, logs.value.length - MAX_LOG_ENTRIES);
  }
  syncScroll();
}

function flushPendingLog(direction: string): void {
  const pending = pendingLogs.get(direction);
  if (!pending || !pending.content) {
    return;
  }
  const content = pending.content;
  const timestamp = pending.timestamp;
  pending.content = "";
  pending.timestamp = "";
  clearPendingTimer(pending);
  pushLogEntry(direction, content, timestamp);
}

function flushAllPendingLogs(): void {
  for (const direction of Array.from(pendingLogs.keys())) {
    flushPendingLog(direction);
  }
}

function clearPendingLogs(): void {
  for (const pending of pendingLogs.values()) {
    clearPendingTimer(pending);
  }
  pendingLogs.clear();
}

function schedulePendingFlush(direction: string): void {
  const pending = ensurePendingLog(direction);
  clearPendingTimer(pending);
  pending.timerId = window.setTimeout(() => {
    flushPendingLog(direction);
  }, LOG_FLUSH_DELAY_MS);
}

function normalizeLogChunk(content: string): string {
  return content.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
}

function handleIncomingLog(payload: SerialConsoleLog): void {
  const pending = ensurePendingLog(payload.direction);
  const normalizedChunk = normalizeLogChunk(payload.content);
  if (!pending.content) {
    pending.timestamp = payload.timestamp;
  }
  pending.content += normalizedChunk;

  let newlineIndex = pending.content.indexOf("\n");
  while (newlineIndex !== -1) {
    const completedLine = pending.content.slice(0, newlineIndex);
    pushLogEntry(payload.direction, completedLine, pending.timestamp || payload.timestamp);
    pending.content = pending.content.slice(newlineIndex + 1);
    pending.timestamp = payload.timestamp;
    newlineIndex = pending.content.indexOf("\n");
  }

  if (pending.content) {
    schedulePendingFlush(payload.direction);
  } else {
    clearPendingTimer(pending);
  }
}

function autoSelectPort(nextPorts: DockPort[]): void {
  if (
    consoleState.value.connected &&
    consoleState.value.portName &&
    nextPorts.some((port) => port.portName === consoleState.value.portName)
  ) {
    selectedPortName.value = consoleState.value.portName;
    return;
  }
  if (
    selectedPortName.value &&
    nextPorts.some((port) => port.portName === selectedPortName.value)
  ) {
    return;
  }
  if (nextPorts.length === 1) {
    selectedPortName.value = nextPorts[0].portName;
    return;
  }
  selectedPortName.value = "";
}

async function refreshPorts(): Promise<void> {
  try {
    const result = await invoke<DockPort[]>("list_serial_console_ports", {
      deviceType: targetDeviceType.value,
    });
    ports.value = result;
    autoSelectPort(result);
    if (!result.length) {
      setStatus(t("serial_console.status_no_port"));
    }
  } catch (error) {
    const message = typeof error === "string" ? error : t("common.unknown_error");
    initError.value = resolveBackendMessage(message);
    setStatus(message);
  }
}

async function loadConsoleState(): Promise<void> {
  try {
    const result = await invoke<SerialConsoleState>("get_serial_console_state");
    applyState(result);
  } catch (error) {
    const message = typeof error === "string" ? error : t("common.unknown_error");
    initError.value = resolveBackendMessage(message);
    setStatus(message);
  }
}

async function connectSelectedPort(): Promise<void> {
  if (!selectedPortName.value) {
    setStatus(t("serial_console.status_pick_port_first"));
    return;
  }
  try {
    await invoke("connect_serial_console", {
      portName: selectedPortName.value,
      deviceType: targetDeviceType.value,
    });
    setStatus(
      t("serial_console.status_connected", {
        port: selectedPortName.value,
      }),
    );
    await loadConsoleState();
    await refreshPorts();
  } catch (error) {
    setStatus(typeof error === "string" ? error : t("common.unknown_error"));
  }
}

async function disconnectPort(): Promise<void> {
  try {
    flushAllPendingLogs();
    await invoke("disconnect_serial_console");
    await loadConsoleState();
    setStatus(t("serial_console.status_disconnected"));
    await refreshPorts();
  } catch (error) {
    setStatus(typeof error === "string" ? error : t("common.unknown_error"));
  }
}

async function sendText(commandOverride?: string): Promise<void> {
  const rawText = commandOverride ?? inputText.value;
  if (!rawText.trim()) {
    setStatus(t("serial_console.status_input_empty"));
    return;
  }
  try {
    await invoke("send_serial_console_text", {
      text: appendNewline.value ? `${rawText}\n` : rawText,
    });
    setStatus(t("serial_console.status_sent"));
    if (!commandOverride) {
      inputText.value = "";
    }
  } catch (error) {
    setStatus(typeof error === "string" ? error : t("common.unknown_error"));
  }
}

function clampRemoteId(raw: number): number {
  if (!Number.isFinite(raw)) return 0;
  return Math.min(255, Math.max(0, Math.floor(raw)));
}

function getRemoteTargetToken(): string {
  if (remoteTargetChoice.value === "all") return "all";
  return String(clampRemoteId(remoteTargetId.value));
}

const paramModalFields = computed((): SerialConsoleField[] => {
  const cmd = paramModalCommand.value;
  if (!cmd || !paramModalOpen.value) return [];
  if (paramModalScope.value === "local") {
    const lc = cmd as SerialConsoleLocalCommand;
    return lc.parametric?.fields ?? [];
  }
  const rc = cmd as SerialConsoleRemoteCommand;
  return rc.parametric?.fields ?? [];
});

const paramPreview = computed(() => {
  const cmd = paramModalCommand.value;
  if (!cmd || !paramModalOpen.value) return { ok: true as const, text: "" };
  if (paramModalScope.value === "local") {
    const lc = cmd as SerialConsoleLocalCommand;
    return tryBuildLocalLine(lc, paramFormValues.value);
  }
  const rc = cmd as SerialConsoleRemoteCommand;
  const tail = tryBuildRemoteTail(rc, paramFormValues.value);
  if (!tail.ok) return tail;
  const tgt = remoteCommandUsesOnlyAll(rc) ? "all" : getRemoteTargetToken();
  return { ok: true as const, text: `send ${tgt} ${tail.text}` };
});

function closeParamModal(): void {
  paramModalOpen.value = false;
  paramModalCommand.value = null;
  paramFormValues.value = {};
}

function openLocalParamModal(cmd: SerialConsoleLocalCommand): void {
  if (!cmd.parametric) return;
  paramModalScope.value = "local";
  paramModalCommand.value = cmd;
  paramFormValues.value = initParamFormValues(cmd.parametric.fields);
  paramModalOpen.value = true;
}

function openRemoteParamModal(cmd: SerialConsoleRemoteCommand): void {
  if (!cmd.parametric) return;
  paramModalScope.value = "remote";
  paramModalCommand.value = cmd;
  paramFormValues.value = initParamFormValues(cmd.parametric.fields);
  paramModalOpen.value = true;
}

async function onQuickCommandClick(cmd: SerialConsoleLocalCommand): Promise<void> {
  if (cmd.parametric) {
    openLocalParamModal(cmd);
    return;
  }
  if (cmd.command) {
    await sendText(cmd.command);
  }
}

async function onRemoteCommandClick(cmd: SerialConsoleRemoteCommand): Promise<void> {
  if (cmd.parametric) {
    openRemoteParamModal(cmd);
    return;
  }
  const tail = cmd.remoteTail;
  if (!tail) return;
  const target = remoteCommandUsesOnlyAll(cmd) ? "all" : getRemoteTargetToken();
  await sendText(`send ${target} ${tail}`);
}

async function submitParamModal(): Promise<void> {
  const cmd = paramModalCommand.value;
  if (!cmd) return;
  const preview = paramPreview.value;
  if (!preview.ok) {
    setStatus(
      preview.messageParams
        ? t(preview.messageKey, preview.messageParams)
        : t(preview.messageKey),
    );
    return;
  }
  await sendText(preview.text);
  closeParamModal();
}

function fillInputFromParamModal(): void {
  const preview = paramPreview.value;
  if (!preview.ok) {
    setStatus(
      preview.messageParams
        ? t(preview.messageKey, preview.messageParams)
        : t(preview.messageKey),
    );
    return;
  }
  if (!preview.text.trim()) {
    setStatus(t("serial_console.status_input_empty"));
    return;
  }
  inputText.value = preview.text;
  closeParamModal();
  setStatus(t("serial_console.status_filled_input"));
}

function localCommandCodeHint(cmd: SerialConsoleLocalCommand): string {
  if (cmd.command) return cmd.command;
  return t("serial_console.parametric_hint");
}

function remoteCommandCodeHint(cmd: SerialConsoleRemoteCommand): string {
  if (cmd.remoteTail) {
    const target = remoteCommandUsesOnlyAll(cmd) ? "all" : getRemoteTargetToken();
    return `send ${target} ${cmd.remoteTail}`;
  }
  return t("serial_console.parametric_hint");
}

function onParamBackdropClick(event: MouseEvent): void {
  if ((event.target as HTMLElement).classList.contains("param-modal-backdrop")) {
    closeParamModal();
  }
}

function onParamModalKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape" && paramModalOpen.value) {
    event.preventDefault();
    closeParamModal();
  }
}

function clearLogs(): void {
  clearPendingLogs();
  logs.value = [];
}

const titleText = computed(() =>
  targetDeviceType.value === "receiver"
    ? t("serial_console.device_receiver")
    : targetTrackerId.value
      ? t("serial_console.device_tracker_slot", { id: targetTrackerId.value })
      : t("serial_console.device_tracker"),
);

const currentConnectionText = computed(() => {
  if (!consoleState.value.connected) {
    return t("serial_console.not_connected");
  }
  const display = consoleState.value.displayName || consoleState.value.portName || "-";
  return t("serial_console.connected_to", { port: display });
});

const quickCommands = computed(() => getSerialConsoleCommands(targetDeviceType.value));
const remoteCommands = computed(() =>
  targetDeviceType.value === "receiver" ? getReceiverRemoteCommands() : [],
);

watch(
  () => targetDeviceType.value,
  async (nextDeviceType, previousDeviceType) => {
    if (nextDeviceType === previousDeviceType) return;
    if (nextDeviceType === "receiver") {
      targetTrackerId.value = null;
    }
    if (!consoleState.value.connected || consoleState.value.deviceType !== nextDeviceType) {
      selectedPortName.value = "";
    }
    await refreshPorts();
  },
);

onMounted(async () => {
  window.addEventListener("keydown", onParamModalKeydown);
  try {
    await loadConsoleState();
    await refreshPorts();
    if (!consoleState.value.connected) {
      setStatus(t("serial_console.status_ready"));
    }
  } catch (error) {
    initError.value = resolveBackendMessage(
      typeof error === "string" ? error : t("common.unknown_error"),
    );
  }

  try {
    unlistenLog = await listen<SerialConsoleLog>("serial-console-log", (event) => {
      handleIncomingLog(event.payload);
    });
  } catch (error) {
    initError.value = resolveBackendMessage(
      typeof error === "string" ? error : t("common.unknown_error"),
    );
  }

  try {
    unlistenState = await listen<SerialConsoleState>("serial-console-state", (event) => {
      const wasConnected = consoleState.value.connected;
      applyState(event.payload);
      if (wasConnected && !event.payload.connected) {
        flushAllPendingLogs();
        setStatus(t("serial_console.status_disconnected"));
      }
    });
  } catch (error) {
    initError.value = resolveBackendMessage(
      typeof error === "string" ? error : t("common.unknown_error"),
    );
  }

  try {
    unlistenTargetHint = await listen<SerialConsoleTargetHint>(
      "serial-console-target-hint",
      async (event) => {
        targetDeviceType.value = event.payload.deviceType;
        targetTrackerId.value = event.payload.trackerId ?? null;
        if (!consoleState.value.connected) {
          selectedPortName.value = "";
        }
        await refreshPorts();
        setStatus(t("serial_console.status_target_updated"));
      },
    );
  } catch (error) {
    initError.value = resolveBackendMessage(
      typeof error === "string" ? error : t("common.unknown_error"),
    );
  }
});

onUnmounted(() => {
  window.removeEventListener("keydown", onParamModalKeydown);
  flushAllPendingLogs();
  clearPendingLogs();
  if (unlistenLog) unlistenLog();
  if (unlistenState) unlistenState();
  if (unlistenTargetHint) unlistenTargetHint();
  invoke("disconnect_serial_console").catch(() => {});
});
</script>

<template>
  <main class="serial-console-page">
    <header class="serial-console-header">
      <div>
        <h1>{{ t("serial_console.title") }}</h1>
        <p>{{ titleText }}</p>
      </div>
      <div class="serial-console-status">
        <span class="status-chip" :class="{ 'status-chip--ok': consoleState.connected }">
          {{ currentConnectionText }}
        </span>
      </div>
    </header>

    <div class="serial-console-layout" :class="{ 'has-remote': remoteCommands.length > 0 }">
      <BasePanel class="panel-connection" :title="t('serial_console.connection_title')">
        <p v-if="initError" class="init-error">
          {{ initError }}
        </p>
        <div class="toolbar-row">
          <BaseSelect v-model="targetDeviceType" class="target-select">
            <option value="tracker">{{ t("serial_console.target_tracker_label") }}</option>
            <option value="receiver">{{ t("serial_console.target_receiver_label") }}</option>
          </BaseSelect>
          <BaseSelect v-model="selectedPortName" class="port-select">
            <option value="">{{ t("serial_console.select_port") }}</option>
            <option v-for="port in ports" :key="port.portName" :value="port.portName">
              {{ port.displayName }}
            </option>
          </BaseSelect>
          <BaseButton variant="outline" @click="refreshPorts">
            {{ t("common.refresh") }}
          </BaseButton>
          <BaseButton
            :disabled="!selectedPortName || consoleState.connected"
            @click="connectSelectedPort"
          >
            {{ t("common.connect") }}
          </BaseButton>
          <BaseButton
            :disabled="!consoleState.connected"
            variant="outline"
            @click="disconnectPort"
          >
            {{ t("common.disconnect") }}
          </BaseButton>
        </div>
        <p class="status-line">{{ statusText }}</p>
      </BasePanel>

      <BasePanel class="panel-log" :title="t('serial_console.log_title')">
        <template #header>
          <div class="panel-head-row">
            <h2 class="panel-head-title">{{ t("serial_console.log_title") }}</h2>
            <BaseButton variant="outline" @click="clearLogs">
              {{ t("common.clear") }}
            </BaseButton>
          </div>
        </template>
        <div ref="logContainer" class="log-list">
          <div
            v-for="(log, index) in logs"
            :key="`${log.timestamp}-${log.direction}-${index}`"
            class="log-item"
            :class="log.direction.toLowerCase()"
            :title="`[${log.timestamp}] ${log.direction}`"
          >
            <span class="log-content"><span v-for="(segment, segmentIndex) in log.segments" :key="segmentIndex" class="log-segment" :class="segment.classes">{{ segment.text }}</span></span>
          </div>
          <div v-if="!logs.length" class="log-empty">
            {{ t("serial_console.log_empty") }}
          </div>
        </div>
      </BasePanel>

      <aside class="panel-sidebar">
        <BasePanel class="panel-input" :title="t('serial_console.input_title')">
          <textarea
            v-model="inputText"
            class="console-input"
            :placeholder="t('serial_console.input_placeholder')"
          />
          <div class="toolbar-row toolbar-row--between">
            <label class="toggle-row">
              <input v-model="appendNewline" type="checkbox" />
              <span>{{ t("serial_console.append_newline") }}</span>
            </label>
            <BaseButton :disabled="!consoleState.connected" @click="() => sendText()">
              {{ t("serial_console.send") }}
            </BaseButton>
          </div>
        </BasePanel>

        <BasePanel class="panel-commands" :title="t('serial_console.quick_commands_title')">
          <div class="command-list-scroll">
            <button
              v-for="command in quickCommands"
              :key="command.key"
              class="command-btn"
              :disabled="!consoleState.connected"
              :title="localCommandCodeHint(command)"
              @click="() => onQuickCommandClick(command)"
            >
              <span class="command-btn-label">{{ t(`serial_console.commands.${command.key}`) }}</span>
              <code class="command-btn-code">{{ localCommandCodeHint(command) }}</code>
            </button>
          </div>
        </BasePanel>
      </aside>

      <aside v-if="remoteCommands.length" class="panel-remote">
        <BasePanel class="panel-commands" :title="t('serial_console.remote_commands_title')">
          <div class="remote-target-row">
            <span class="remote-target-label">{{ t("serial_console.remote_target_label") }}</span>
            <BaseSelect v-model="remoteTargetChoice" class="remote-target-mode">
              <option value="all">{{ t("serial_console.remote_target_all") }}</option>
              <option value="id">{{ t("serial_console.remote_target_id") }}</option>
            </BaseSelect>
            <input
              v-show="remoteTargetChoice === 'id'"
              v-model.number="remoteTargetId"
              class="remote-target-id-input"
              type="number"
              min="0"
              max="255"
              step="1"
              :aria-label="t('serial_console.remote_target_id')"
            />
          </div>
          <p v-if="remoteTargetChoice === 'id'" class="remote-target-hint">
            {{ t("serial_console.remote_target_hint_id") }}
          </p>
          <div class="command-list-scroll">
            <button
              v-for="command in remoteCommands"
              :key="command.key"
              class="command-btn"
              :disabled="!consoleState.connected"
              :title="remoteCommandCodeHint(command)"
              @click="() => onRemoteCommandClick(command)"
            >
              <span class="command-btn-label">{{ t(`serial_console.commands.${command.key}`) }}</span>
              <code class="command-btn-code">{{ remoteCommandCodeHint(command) }}</code>
            </button>
          </div>
        </BasePanel>
      </aside>
    </div>

    <div
      v-if="paramModalOpen"
      class="param-modal-backdrop"
      role="presentation"
      @click="onParamBackdropClick"
    >
      <div
        class="param-modal"
        role="dialog"
        aria-modal="true"
        :aria-label="t('serial_console.param_modal_title')"
        @click.stop
      >
        <h3 class="param-modal-title">{{ t("serial_console.param_modal_title") }}</h3>
        <p v-if="paramModalScope === 'remote'" class="param-modal-sub">
          {{ t("serial_console.param_modal_remote_note") }}
        </p>
        <form class="param-modal-form" @submit.prevent="submitParamModal">
          <div
            v-for="field in paramModalFields"
            :key="field.id"
            class="param-field"
          >
            <label
              v-if="field.kind !== 'toggle'"
              class="param-field-label"
              :for="`param-${field.id}`"
              >{{ t(field.labelKey) }}</label
            >
            <template v-if="field.kind === 'text'">
              <input
                :id="`param-${field.id}`"
                v-model="paramFormValues[field.id] as string"
                class="param-field-control"
                type="text"
                :placeholder="
                  field.placeholderKey ? t(field.placeholderKey) : undefined
                "
                autocomplete="off"
              />
            </template>
            <template v-else-if="field.kind === 'number'">
              <input
                :id="`param-${field.id}`"
                v-model="paramFormValues[field.id]"
                class="param-field-control"
                type="number"
                :min="field.min"
                :max="field.max"
                :step="field.integer ? 1 : 'any'"
              />
            </template>
            <template v-else-if="field.kind === 'select'">
              <BaseSelect
                :id="`param-${field.id}`"
                v-model="paramFormValues[field.id] as string"
                class="param-field-control param-field-select"
              >
                <option
                  v-for="opt in field.options"
                  :key="opt.value"
                  :value="opt.value"
                >
                  {{ t(opt.labelKey) }}
                </option>
              </BaseSelect>
            </template>
            <template v-else-if="field.kind === 'toggle'">
              <label class="param-toggle-row" :for="`param-${field.id}`">
                <input
                  :id="`param-${field.id}`"
                  v-model="paramFormValues[field.id] as boolean"
                  type="checkbox"
                />
                <span>{{ t(field.labelKey) }}</span>
              </label>
            </template>
          </div>
          <div class="param-preview">
            <span class="param-preview-label">{{ t("serial_console.param_preview") }}</span>
            <code class="param-preview-code">{{
              paramPreview.ok ? paramPreview.text : "—"
            }}</code>
            <p v-if="!paramPreview.ok" class="param-preview-error">
              {{
                paramPreview.messageParams
                  ? t(paramPreview.messageKey, paramPreview.messageParams)
                  : t(paramPreview.messageKey)
              }}
            </p>
          </div>
          <div class="param-modal-actions">
            <BaseButton type="button" variant="outline" @click="closeParamModal">
              {{ t("serial_console.param_modal_cancel") }}
            </BaseButton>
            <BaseButton type="button" variant="outline" @click="fillInputFromParamModal">
              {{ t("serial_console.fill") }}
            </BaseButton>
            <BaseButton type="submit" :disabled="!consoleState.connected">
              {{ t("serial_console.send_now") }}
            </BaseButton>
          </div>
        </form>
      </div>
    </div>
  </main>
</template>

<style scoped>
.serial-console-page {
  height: 100vh;
  max-height: 100vh;
  overflow: hidden;
  background: var(--color-bg-page);
  color: var(--color-text-main);
  padding: var(--spacing-md);
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
}

.serial-console-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--spacing-md);
  margin-bottom: var(--spacing-md);
}

.serial-console-header h1 {
  margin: 0 0 var(--spacing-xs);
  font-size: 22px;
}

.serial-console-header p {
  margin: 0;
  color: var(--color-text-light);
}

.status-chip {
  display: inline-flex;
  align-items: center;
  min-height: 32px;
  padding: 0 var(--spacing-sm);
  border: var(--border-width) solid var(--color-secondary);
  background: var(--color-bg-header);
  color: var(--color-text-light);
}

.status-chip--ok {
  border-color: var(--color-success-border);
  color: var(--color-success-border);
  background: var(--color-success-bg);
}

.serial-console-layout {
  display: grid;
  grid-template-columns: minmax(300px, 1fr) 300px;
  grid-template-rows: auto minmax(0, 1fr);
  grid-template-areas:
    "connection connection"
    "log sidebar";
  gap: var(--spacing-md);
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.serial-console-layout.has-remote {
  grid-template-columns: minmax(300px, 1fr) 280px 280px;
  grid-template-areas:
    "connection connection connection"
    "log sidebar remote";
}

.panel-connection {
  grid-area: connection;
}

.panel-log {
  grid-area: log;
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.panel-sidebar {
  grid-area: sidebar;
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
}

.panel-sidebar > :deep(.base-panel) {
  margin-bottom: 0;
}

.panel-remote {
  grid-area: remote;
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.panel-remote > :deep(.base-panel) {
  margin-bottom: 0;
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.panel-input {
  flex-shrink: 0;
}

.panel-commands {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.panel-commands :deep(.panel-content) {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.panel-log :deep(.panel-content) {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.panel-input :deep(.panel-content) {
  display: flex;
  flex-direction: column;
}

.toolbar-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--spacing-sm);
}

.toolbar-row--between {
  justify-content: space-between;
}

.port-select {
  min-width: min(420px, 100%);
  flex: 1 1 320px;
}

.target-select {
  min-width: 180px;
  flex: 0 0 180px;
}

.status-line {
  margin: var(--spacing-sm) 0 0;
  color: var(--color-text-light);
}

.init-error {
  margin: 0 0 var(--spacing-sm);
  color: var(--color-error);
  font-weight: 700;
}

.panel-head-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--spacing-sm);
}

.panel-head-title {
  margin: 0;
  font-size: 18px;
  color: var(--color-text-secondary);
}

.log-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  background: #0d1117;
  color: #c9d1d9;
  border: var(--border-width) solid var(--color-secondary);
  padding: 0;
  font-family: var(--font-family-mono);
  font-size: 12px;
  line-height: 1.5;
}

.log-item {
  display: flex;
  align-items: stretch;
  padding: 0;
  border-left: 3px solid transparent;
}

.log-item.rx {
  border-left-color: #3fb950;
}

.log-item.tx {
  border-left-color: #d29922;
  background: rgba(210, 153, 34, 0.06);
}

.log-item.sys {
  border-left-color: #58a6ff;
  background: rgba(88, 166, 255, 0.06);
}

.log-item.tx + .log-item.rx,
.log-item.sys + .log-item.rx,
.log-item.rx + .log-item.tx,
.log-item.rx + .log-item.sys,
.log-item.tx + .log-item.sys,
.log-item.sys + .log-item.tx {
  margin-top: 2px;
}

.log-content {
  display: block;
  flex: 1;
  min-height: 1.2em;
  margin: 0;
  padding: 0 8px;
  white-space: pre-wrap;
  word-break: break-word;
  color: inherit;
  font-family: var(--font-family-mono);
  font-size: inherit;
  line-height: inherit;
}

.log-item.tx .log-content {
  color: #e3b341;
}

.log-item.sys .log-content {
  color: #8b949e;
  font-style: italic;
}

.log-segment {
  color: inherit;
}

.ansi-bold {
  font-weight: 700;
}

.ansi-dim {
  opacity: 0.7;
}

.ansi-italic {
  font-style: italic;
}

.ansi-underline {
  text-decoration: underline;
}

.ansi-fg-black {
  color: #2f3338;
}

.ansi-fg-red {
  color: #ff7b72;
}

.ansi-fg-green {
  color: #7ee787;
}

.ansi-fg-yellow {
  color: #f2cc60;
}

.ansi-fg-blue {
  color: #79c0ff;
}

.ansi-fg-magenta {
  color: #d2a8ff;
}

.ansi-fg-cyan {
  color: #76e3ea;
}

.ansi-fg-white {
  color: #f0f6fc;
}

.ansi-fg-bright-black {
  color: #8b949e;
}

.ansi-fg-bright-red {
  color: #ffa198;
}

.ansi-fg-bright-green {
  color: #56d364;
}

.ansi-fg-bright-yellow {
  color: #e3b341;
}

.ansi-fg-bright-blue {
  color: #a5d6ff;
}

.ansi-fg-bright-magenta {
  color: #e2b8ff;
}

.ansi-fg-bright-cyan {
  color: #b3f0ff;
}

.ansi-fg-bright-white {
  color: #ffffff;
}

.ansi-bg-black {
  background: #2f3338;
}

.ansi-bg-red {
  background: rgba(255, 123, 114, 0.2);
}

.ansi-bg-green {
  background: rgba(126, 231, 135, 0.2);
}

.ansi-bg-yellow {
  background: rgba(242, 204, 96, 0.2);
}

.ansi-bg-blue {
  background: rgba(121, 192, 255, 0.2);
}

.ansi-bg-magenta {
  background: rgba(210, 168, 255, 0.2);
}

.ansi-bg-cyan {
  background: rgba(118, 227, 234, 0.2);
}

.ansi-bg-white {
  background: rgba(240, 246, 252, 0.2);
}

.ansi-bg-bright-black {
  background: rgba(139, 148, 158, 0.2);
}

.ansi-bg-bright-red {
  background: rgba(255, 161, 152, 0.2);
}

.ansi-bg-bright-green {
  background: rgba(86, 211, 100, 0.2);
}

.ansi-bg-bright-yellow {
  background: rgba(227, 179, 65, 0.2);
}

.ansi-bg-bright-blue {
  background: rgba(165, 214, 255, 0.2);
}

.ansi-bg-bright-magenta {
  background: rgba(226, 184, 255, 0.2);
}

.ansi-bg-bright-cyan {
  background: rgba(179, 240, 255, 0.2);
}

.ansi-bg-bright-white {
  background: rgba(255, 255, 255, 0.2);
}

.log-empty {
  color: #9ba9bb;
}

.console-input {
  width: 100%;
  min-height: 80px;
  max-height: 160px;
  padding: var(--spacing-sm);
  box-sizing: border-box;
  resize: vertical;
  border: var(--border-width) solid var(--color-border-control);
  border-radius: var(--border-radius);
  font: inherit;
  font-family: var(--font-family-mono);
  margin-bottom: var(--spacing-sm);
}

.console-input:focus {
  outline: none;
  border-color: var(--color-primary);
}

.toggle-row {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-xs);
  color: var(--color-text-light);
}

.command-list-scroll {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding-right: 2px;
}

.command-btn {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  width: 100%;
  padding: 6px var(--spacing-sm);
  border: var(--border-width) solid var(--color-secondary);
  background: var(--color-bg-white);
  color: var(--color-text-main);
  cursor: pointer;
  text-align: left;
  font: inherit;
  transition: background 0.15s, border-color 0.15s;
}

.command-btn:hover:not(:disabled) {
  background: var(--color-bg-header);
  border-color: var(--color-primary);
}

.command-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.command-btn-label {
  font-weight: 600;
  white-space: normal;
  min-width: 0;
  flex: 1 1 auto;
  overflow-wrap: anywhere;
}

.command-btn-code {
  font-family: var(--font-family-mono);
  font-size: 12px;
  color: var(--color-text-light);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.remote-target-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--spacing-sm);
  margin-bottom: var(--spacing-xs);
}

.remote-target-label {
  font-size: 13px;
  color: var(--color-text-light);
  flex: 0 0 auto;
}

.remote-target-mode {
  flex: 1 1 120px;
  min-width: 100px;
  max-width: 160px;
}

.remote-target-id-input {
  width: 72px;
  min-height: 32px;
  padding: 6px 8px;
  border: var(--border-width) solid var(--color-border-control);
  border-radius: var(--border-radius);
  font: inherit;
  box-sizing: border-box;
}

.remote-target-hint {
  margin: 0 0 var(--spacing-sm);
  font-size: 12px;
  color: var(--color-text-light);
}

.param-modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 10000;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-md);
  box-sizing: border-box;
}

.param-modal {
  width: min(440px, 100%);
  max-height: min(90vh, 640px);
  overflow: auto;
  background: var(--color-bg-white);
  color: var(--color-text-main);
  border: var(--border-width) solid var(--color-secondary);
  border-radius: var(--border-radius);
  padding: var(--spacing-md);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
}

.param-modal-title {
  margin: 0 0 var(--spacing-xs);
  font-size: 18px;
}

.param-modal-sub {
  margin: 0 0 var(--spacing-sm);
  font-size: 13px;
  color: var(--color-text-light);
}

.param-modal-form {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
}

.param-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.param-field-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text-secondary);
}

.param-field-control {
  width: 100%;
  min-height: 32px;
  padding: 6px 10px;
  border: var(--border-width) solid var(--color-border-control);
  border-radius: var(--border-radius);
  font: inherit;
  box-sizing: border-box;
}

.param-field-control:focus {
  outline: none;
  border-color: var(--color-primary);
}

.param-field-select {
  padding: 0;
}

.param-field-select :deep(.base-select) {
  width: 100%;
}

.param-toggle-row {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-xs);
  font-size: 14px;
  cursor: pointer;
  color: var(--color-text-main);
}

.param-preview {
  margin-top: var(--spacing-xs);
  padding: var(--spacing-sm);
  background: var(--color-bg-header);
  border: var(--border-width) solid var(--color-border-control);
  border-radius: var(--border-radius);
}

.param-preview-label {
  display: block;
  font-size: 12px;
  color: var(--color-text-light);
  margin-bottom: 4px;
}

.param-preview-code {
  display: block;
  font-family: var(--font-family-mono);
  font-size: 13px;
  word-break: break-all;
  white-space: pre-wrap;
}

.param-preview-error {
  margin: var(--spacing-xs) 0 0;
  font-size: 12px;
  color: var(--color-error);
}

.param-modal-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: var(--spacing-sm);
  margin-top: var(--spacing-sm);
}

</style>
