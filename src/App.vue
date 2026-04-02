<script setup lang="ts">
import {
  computed,
  inject,
  markRaw,
  onMounted,
  onUnmounted,
  ref,
  shallowRef,
  type Ref,
} from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { check } from "@tauri-apps/plugin-updater";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useI18n } from "vue-i18n";

// 导入组件
import Home from "./views/Home.vue";
import TrackerFlashing from "./views/TrackerFlashing.vue";
import Settings from "./views/Settings.vue";
import NotificationManager from "./components/NotificationManager.vue";
import BaseSpinner from "./components/ui/BaseSpinner.vue";
import WindowTitleBar from "./components/ui/WindowTitleBar.vue";
import DebugConsole from "./components/DebugConsole.vue";
import {
  SYSTEM_SETTINGS_INJECTION_KEY,
  type LanguagePreference,
  type SystemSettings,
} from "./types/settings";
import { resolveLocaleFromPreference } from "./utils/locale";
import type { DockInfo, DockPort, TrackerStatus } from "./types/dock";
import type {
  FirmwareFile,
  FirmwareFlashResult,
  FirmwareMode,
  FirmwarePhase,
  FirmwareProgressEvent,
  FirmwareRunItem,
  FirmwareRunState,
} from "./types/firmware";

const { t, locale } = useI18n();
const appWindow = getCurrentWindow();

/** main.ts 中 provide；若缺失会在运行时报错 */
const systemSettings = inject(SYSTEM_SETTINGS_INJECTION_KEY) as Ref<SystemSettings>;

// --- 仅 App 内部使用的类型 ---
type UsbTopologyResult = {
  id: number;
  inserted: boolean;
  usb_path?: string;
  tracker_version?: string;
};

/** Tauri 后端 serde 默认 snake_case，与前端 TrackerStatus 字段对齐用 */
type TrackerStatusWire = TrackerStatus & {
  usb_path?: string;
  tracker_version?: string;
};

type AckResponse = {
  cmd: string;
  success: boolean;
  msg?: string;
};

type DockStatusResponse = {
  led?: boolean;
  bl_mode?: number;
  auto_sleep?: boolean;
  trackers: TrackerStatusWire[];
};

type BlModeResponse = {
  mode: number;
  name?: string;
};

type AutoSleepResponse = {
  enabled: boolean;
};

type DockWindowsResult = {
  slimevrFound: boolean;
  slimevrDocked: boolean;
};

type WindowDockingConfig = {
  autoDockOnStartup: boolean;
  dockAlwaysOnTop: boolean;
  followSlimeVrWindow: boolean;
  snapStyleApproximation: boolean;
};

interface Notification {
  id: number;
  message: string;
  type: 'info' | 'success' | 'error';
  timestamp: number;
}

// --- 路由与视图控制 ---
const searchParams = new URLSearchParams(window.location.search);
const isDebugWindow = ref(searchParams.get("debug") === "true");
const currentView = ref<'home' | 'flashing' | 'settings'>('home');

// --- 状态定义 ---
const docks = ref<DockPort[]>([]);
const connectedPortName = ref("");
const dockInfo = ref<DockInfo | null>(null);
const trackers = ref<TrackerStatus[]>(
  Array.from({ length: 10 }, (_, index) => ({ id: index + 1, inserted: false })),
);
const ledEnabled = ref(false);
const loading = ref(false);
const showOverlay = ref(false); // 控制全屏遮罩
const elapsedTime = ref(0); // 当前已执行时间 (s)
const estimatedTime = ref(0); // 预计总时间 (s)
const notifications = ref<Notification[]>([]);
const blMode = ref<number | null>(null);
const blModeName = ref("");
const autoSleepEnabled = ref(false);
const appUpdateChecking = ref(false);
const appUpdateInstalling = ref(false);
const appUpdateAvailable = ref(false);
const appUpdateVersion = ref<string | null>(null);
const appUpdateStatusKey = ref("settings.update_status_idle");
const appUpdateStatusParams = ref<Record<string, unknown>>({});
const appUpdateStatusText = computed(() =>
  t(appUpdateStatusKey.value, appUpdateStatusParams.value),
);
/** Tauri Update 为带私有字段的类实例，勿放入深层响应式 ref，否则 downloadAndInstall 会报私有成员错误 */
const pendingAppUpdate = shallowRef<Awaited<ReturnType<typeof check>> | null>(null);
const windowDockingBusy = ref(false);
const windowAlwaysOnTop = ref(false);
const firmwareBusy = ref(false);
const firmwareTrackerId = ref(1);
const firmwareFile = ref<FirmwareFile | null>(null);
const firmwarePhase = ref<FirmwarePhase>("idle");
const firmwareProgress = ref(0);
const firmwareStatusMessage = ref(t("flashing.idle_status"));
const firmwareMode = ref<FirmwareMode>("manual");
const autoUpdateEnabled = ref(false);
const autoPendingTrackerId = ref<number | null>(null);
const activeFirmwareTrackerId = ref<number | null>(null);
const firmwareRunItems = ref<Record<number, FirmwareRunItem>>(createFirmwareRunItems());
const blModeOptions = computed(() => [
  { mode: 0, label: t("tracker_control.bl_mode_option_0") },
  { mode: 1, label: t("tracker_control.bl_mode_option_1") },
]);
const uiBusy = computed(() => loading.value || firmwareBusy.value);
const selectedFirmwareTracker = computed(
  () => trackers.value.find((tracker) => tracker.id === firmwareTrackerId.value) ?? null,
);
const firmwareSlotStatuses = computed(() =>
  Array.from({ length: 5 }, (_, index) => {
    const id = index + 1;
    const tracker = trackers.value.find((item) => item.id === id);
    const runItem = firmwareRunItems.value[id];
    return {
      id,
      inserted: tracker?.inserted ?? false,
      usbPath: tracker?.usbPath ?? "",
      state: runItem?.state ?? "idle",
      message: runItem?.message ?? t("flashing.slot_state_idle"),
    };
  }),
);

// --- 指令延时定义 (s) ---
const ACTION_DELAYS: Record<string, number> = {
  'ret': 0.5,
  'ret_all': 0.5,
  'bl': 1.0,
  'bl_all': 1.0,
  'sleep': 1.5,
  'sleep_all': 1.5,
  'pair': 6.5,
  'pair_all': 6.5,
  'connect': 3.0 // 连接操作预估时间
};

let nextNotifyId = 0;
let overlayTimer: number | null = null;
let connectionMonitorTimer: number | null = null;
const BACKEND_I18N_PREFIX = "i18n:";

function startOverlayTimer(action: string) {
  elapsedTime.value = 0;
  estimatedTime.value = ACTION_DELAYS[action] || 0;
  showOverlay.value = true;
  
  const start = Date.now();
  overlayTimer = window.setInterval(() => {
    elapsedTime.value = (Date.now() - start) / 1000;
  }, 100);
}

function stopOverlayTimer() {
  if (overlayTimer) {
    clearInterval(overlayTimer);
    overlayTimer = null;
  }
  showOverlay.value = false;
  elapsedTime.value = 0;
  estimatedTime.value = 0;
}
let unlistenDock: (() => void) | null = null;
let unlistenFirmware: (() => void) | null = null;

// --- 通知逻辑 ---
function addNotification(message: string, type: 'info' | 'success' | 'error' = 'info') {
  const id = nextNotifyId++;
  notifications.value.push({ id, message, type, timestamp: Date.now() });
  setTimeout(() => {
    notifications.value = notifications.value.filter(n => n.id !== id);
  }, 5000);
}

function pushLog(message: string, type: 'info' | 'success' | 'error' = 'info'): void {
  addNotification(message, type);
}

function setAppUpdateStatus(key: string, params: Record<string, unknown> = {}): void {
  appUpdateStatusKey.value = key;
  appUpdateStatusParams.value = params;
}

function resetConnectedState(): void {
  connectedPortName.value = "";
  dockInfo.value = null;
  trackers.value = normalizeTrackers([]);
  ledEnabled.value = false;
  blMode.value = null;
  blModeName.value = "";
  autoSleepEnabled.value = false;
  resetFirmwareState({ keepFile: true });
}

function resolveBackendI18nMessage(raw: string): string | null {
  if (!raw.startsWith(BACKEND_I18N_PREFIX)) return null;
  const payload = raw.slice(BACKEND_I18N_PREFIX.length);
  const separatorIndex = payload.indexOf("|");
  const key = separatorIndex === -1 ? payload : payload.slice(0, separatorIndex);
  if (!key) return null;
  if (separatorIndex === -1) {
    return t(key);
  }
  const paramsText = payload.slice(separatorIndex + 1);
  try {
    const parsed = JSON.parse(paramsText) as Record<string, unknown>;
    const resolvedParams = Object.fromEntries(
      Object.entries(parsed).map(([paramKey, paramValue]) => {
        if (typeof paramValue === "string") {
          return [paramKey, resolveBackendI18nMessage(paramValue) ?? paramValue];
        }
        return [paramKey, paramValue];
      }),
    );
    return t(key, resolvedParams);
  } catch {
    return t(key);
  }
}

// --- 核心业务逻辑 ---
function getErrorMessage(error: unknown): string {
  const rawMessage = typeof error === "string"
    ? error
    : error instanceof Error
      ? error.message
      : "";
  if (rawMessage) {
    return resolveBackendI18nMessage(rawMessage) ?? rawMessage;
  }
  return t('common.unknown_error');
}

function resolveMessage(raw: string): string {
  return resolveBackendI18nMessage(raw) ?? raw;
}

function createFirmwareRunItems(): Record<number, FirmwareRunItem> {
  return Object.fromEntries(
    Array.from({ length: 5 }, (_, index) => [
      index + 1,
      { state: "idle", message: t("flashing.slot_state_idle") },
    ]),
  ) as Record<number, FirmwareRunItem>;
}

function resetFirmwareRunItems(): void {
  firmwareRunItems.value = createFirmwareRunItems();
}

function setFirmwareRunItem(id: number, state: FirmwareRunState, message: string): void {
  if (id < 1 || id > 5) return;
  firmwareRunItems.value = {
    ...firmwareRunItems.value,
    [id]: { state, message },
  };
}

function getTrackerById(id: number): TrackerStatus | undefined {
  return trackers.value.find((tracker) => tracker.id === id);
}

function armAutoUpdateWaitingState(): void {
  autoPendingTrackerId.value = null;
  resetFirmwareRunItems();
  if (firmwareMode.value === "auto_slot" && autoUpdateEnabled.value) {
    const message = t("flashing.auto_armed_status", { id: firmwareTrackerId.value });
    setFirmwareRunItem(firmwareTrackerId.value, "waiting", message);
    firmwareProgress.value = 0;
    firmwarePhase.value = firmwareFile.value ? "ready" : "idle";
    firmwareStatusMessage.value = message;
  }
}

function resetFirmwareState(options: { keepFile?: boolean } = {}): void {
  if (!options.keepFile) {
    firmwareFile.value = null;
  }
  firmwareBusy.value = false;
  firmwareMode.value = "manual";
  autoUpdateEnabled.value = false;
  autoPendingTrackerId.value = null;
  activeFirmwareTrackerId.value = null;
  resetFirmwareRunItems();
  firmwareProgress.value = 0;
  if (options.keepFile && firmwareFile.value) {
    firmwarePhase.value = "ready";
    firmwareStatusMessage.value = t("flashing.file_loaded_status", { name: firmwareFile.value.name });
  } else {
    firmwarePhase.value = "idle";
    firmwareStatusMessage.value = t("flashing.idle_status");
  }
}

function normalizeTrackers(current: TrackerStatusWire[]): TrackerStatus[] {
  const map = new Map(current.map((item) => [item.id, item]));
  return Array.from({ length: 10 }, (_, index) => {
    const id = index + 1;
    const existing = trackers.value.find(t => t.id === id);
    const newData = map.get(id);
    const usbPath =
      newData?.usbPath ?? newData?.usb_path ?? existing?.usbPath;
    const trackerVersion =
      newData?.trackerVersion ??
      newData?.tracker_version ??
      existing?.trackerVersion;
    return {
      id,
      inserted: newData?.inserted ?? false,
      usbPath,
      trackerVersion,
    };
  });
}

async function refreshDocks(): Promise<void> {
  loading.value = true;
  try {
    const result = await invoke<DockPort[]>("discover_docks");
    docks.value = result;
    pushLog(t('notifications.scan_found', { count: result.length }), 'info');
  } catch (error) {
    pushLog(t('notifications.scan_failed', { msg: getErrorMessage(error) }), 'error');
  } finally {
    loading.value = false;
  }
}

async function loadConnectedPort(): Promise<void> {
  try {
    const current = await invoke<string | null>("get_connected_port");
    connectedPortName.value = current ?? "";
  } catch (error) {
    pushLog(t('notifications.status_read_failed', { msg: getErrorMessage(error) }), 'error');
  }
}

async function connectDock(portName: string): Promise<void> {
  if (connectedPortName.value) {
    pushLog(t('notifications.already_connected', { name: connectedPortName.value }), 'info');
    return;
  }
  if (!portName) {
    pushLog(t('notifications.select_port_first'), 'info');
    return;
  }
  loading.value = true;
  startOverlayTimer('connect');
  try {
    const dock = await invoke<DockPort>("connect_dock", { portName });
    connectedPortName.value = dock.portName;
    pushLog(t('notifications.connect_success', { name: dock.displayName }), 'success');
    await refreshDockInfo();
    await refreshTrackerStatus();
    await refreshBlMode();
    await refreshAutoSleep();
  } catch (error) {
    pushLog(t('notifications.connect_failed', { msg: getErrorMessage(error) }), 'error');
  } finally {
    loading.value = false;
    stopOverlayTimer();
  }
}

async function disconnectDock(): Promise<void> {
  loading.value = true;
  try {
    await invoke("disconnect_dock");
    resetConnectedState();
    pushLog(t('notifications.disconnect_success'), 'info');
  } catch (error) {
    pushLog(t('notifications.disconnect_failed', { msg: getErrorMessage(error) }), 'error');
  } finally {
    loading.value = false;
  }
}

async function checkDockConnectionHealth(): Promise<void> {
  if (!connectedPortName.value || loading.value || firmwareBusy.value) return;
  try {
    const connected = await invoke<boolean>("check_dock_connection");
    if (!connected && connectedPortName.value) {
      resetConnectedState();
      pushLog(t('notifications.usb_disconnected'), 'error');
    }
  } catch {
  }
}

async function refreshDockInfo(): Promise<void> {
  if (!connectedPortName.value) return;
  try {
    dockInfo.value = await invoke<DockInfo>("get_dock_info");
  } catch (error) {
    pushLog(t('notifications.info_read_failed', { msg: getErrorMessage(error) }), 'error');
  }
}

async function refreshTrackerStatus(): Promise<void> {
  if (!connectedPortName.value) return;
  try {
    const result = await invoke<DockStatusResponse>("get_dock_status");
    trackers.value = normalizeTrackers(result.trackers);
    if (typeof result.led === "boolean") {
      ledEnabled.value = result.led;
    }
    if (typeof result.bl_mode === "number") {
      blMode.value = result.bl_mode;
    }
    if (typeof result.auto_sleep === "boolean") {
      autoSleepEnabled.value = result.auto_sleep;
    }
    await scanUsbTopology();
  } catch (error) {
    pushLog(t('notifications.tracker_status_failed', { msg: getErrorMessage(error) }), 'error');
  }
}

async function refreshBlMode(): Promise<void> {
  if (!connectedPortName.value) return;
  try {
    const result = await invoke<BlModeResponse>("get_bl_mode");
    blMode.value = result.mode;
    blModeName.value = result.name ?? "";
  } catch (error) {
    pushLog(t('notifications.bl_mode_read_failed', { msg: getErrorMessage(error) }), 'error');
  }
}

async function refreshAutoSleep(): Promise<void> {
  if (!connectedPortName.value) return;
  try {
    const result = await invoke<AutoSleepResponse>("get_auto_sleep");
    autoSleepEnabled.value = result.enabled;
  } catch (error) {
    pushLog(t('notifications.auto_sleep_read_failed', { msg: getErrorMessage(error) }), 'error');
  }
}

let scanTimeout: number | null = null;
const EVENT_FLAP_WINDOW_MS = 2500;
let topologyScanBlockedUntil = 0;
const trackerLastEvent = new Map<number, { inserted: boolean; ts: number }>();

function markTrackerEvent(id: number, inserted: boolean): void {
  const now = Date.now();
  const prev = trackerLastEvent.get(id);
  if (prev && prev.inserted !== inserted && now - prev.ts <= EVENT_FLAP_WINDOW_MS) {
    topologyScanBlockedUntil = Math.max(topologyScanBlockedUntil, now + EVENT_FLAP_WINDOW_MS);
    if (scanTimeout) {
      clearTimeout(scanTimeout);
      scanTimeout = null;
    }
  }
  trackerLastEvent.set(id, { inserted, ts: now });
}

async function scanUsbTopology(): Promise<void> {
  if (!connectedPortName.value) return;
  if (Date.now() < topologyScanBlockedUntil) return;
  if (scanTimeout) clearTimeout(scanTimeout);

  scanTimeout = window.setTimeout(async () => {
    try {
      if (Date.now() < topologyScanBlockedUntil) return;
      const usbResults = await invoke<UsbTopologyResult[]>("scan_usb_topology");
      trackers.value = trackers.value.map(t => {
        const usbInfo = usbResults.find(u => u.id === t.id);
        return {
          ...t,
          usbPath: usbInfo?.usb_path ?? t.usbPath,
          trackerVersion: usbInfo?.tracker_version ?? t.trackerVersion,
        };
      });
      void maybeTriggerAutoFirmwareUpdate();
    } catch (error) {
      console.error("[USB] Topology scan failed:", error);
    } finally {
      scanTimeout = null;
    }
  }, 2000);
}

function setFirmwareMode(mode: FirmwareMode): void {
  if (firmwareBusy.value) return;
  firmwareMode.value = mode;
  activeFirmwareTrackerId.value = null;
  autoPendingTrackerId.value = null;
  resetFirmwareRunItems();
  if (mode !== "auto_slot") {
    autoUpdateEnabled.value = false;
  }
  firmwareProgress.value = 0;
  firmwarePhase.value = firmwareFile.value ? "ready" : "idle";
  firmwareStatusMessage.value = firmwareFile.value
    ? t("flashing.file_loaded_status", { name: firmwareFile.value.name })
    : t("flashing.idle_status");
}

function setFirmwareTrackerId(id: number): void {
  firmwareTrackerId.value = Math.min(5, Math.max(1, id));
  if (firmwareMode.value === "auto_slot" && autoUpdateEnabled.value) {
    armAutoUpdateWaitingState();
  } else if (!firmwareBusy.value) {
    resetFirmwareRunItems();
  }
}

async function selectFirmwareFile(file: File | null): Promise<void> {
  if (!file) return;
  if (!file.name.toLowerCase().endsWith(".uf2")) {
    firmwareFile.value = null;
    firmwarePhase.value = "error";
    firmwareProgress.value = 0;
    firmwareStatusMessage.value = t("flashing.invalid_file_type");
    pushLog(t("flashing.invalid_file_type"), "error");
    return;
  }
  try {
    const bytes = new Uint8Array(await file.arrayBuffer());
    if (bytes.length === 0) {
      firmwareFile.value = null;
      firmwarePhase.value = "error";
      firmwareProgress.value = 0;
      firmwareStatusMessage.value = t("flashing.empty_file");
      pushLog(t("flashing.empty_file"), "error");
      return;
    }
    firmwareFile.value = {
      name: file.name,
      size: file.size,
      bytes,
    };
    resetFirmwareRunItems();
    firmwarePhase.value = "ready";
    firmwareProgress.value = 0;
    firmwareStatusMessage.value = t("flashing.file_loaded_status", { name: file.name });
    if (firmwareMode.value === "auto_slot" && autoUpdateEnabled.value) {
      armAutoUpdateWaitingState();
    }
  } catch (error) {
    const message = getErrorMessage(error);
    firmwareFile.value = null;
    firmwarePhase.value = "error";
    firmwareProgress.value = 0;
    firmwareStatusMessage.value = message;
    pushLog(message, "error");
  }
}

async function runFirmwareFlashForTracker(trackerId: number): Promise<FirmwareFlashResult> {
  if (!connectedPortName.value) {
    throw new Error(t("flashing.require_connection"));
  }
  if (!firmwareFile.value) {
    throw new Error(t("flashing.require_file"));
  }
  const tracker = getTrackerById(trackerId);
  if (!tracker?.inserted) {
    throw new Error(t("flashing.require_inserted_tracker", { id: trackerId }));
  }

  firmwareBusy.value = true;
  activeFirmwareTrackerId.value = trackerId;
  firmwarePhase.value = "entering_bl";
  firmwareProgress.value = 0;
  firmwareStatusMessage.value = t("flashing.progress_entering_bl", { id: trackerId });
  setFirmwareRunItem(trackerId, "running", firmwareStatusMessage.value);

  try {
    const result = await invoke<FirmwareFlashResult>("flash_tracker_firmware", {
      trackerId,
      fileName: firmwareFile.value.name,
      fileData: Array.from(firmwareFile.value.bytes),
    });
    const message = resolveMessage(result.message);
    firmwarePhase.value = result.phase;
    firmwareProgress.value = result.progress;
    firmwareStatusMessage.value = message;
    setFirmwareRunItem(trackerId, result.warning ? "warning" : "success", message);
    return result;
  } catch (error) {
    const message = getErrorMessage(error);
    firmwarePhase.value = "error";
    firmwareStatusMessage.value = message;
    setFirmwareRunItem(trackerId, "error", message);
    pushLog(message, "error");
    throw error;
  } finally {
    firmwareBusy.value = false;
    activeFirmwareTrackerId.value = null;
    await refreshTrackerStatus();
  }
}

async function startFirmwareFlash(): Promise<void> {
  if (firmwareBusy.value) return;
  resetFirmwareRunItems();
  try {
    await runFirmwareFlashForTracker(firmwareTrackerId.value);
  } catch {
  }
}

async function toggleAutoUpdate(): Promise<void> {
  if (firmwareBusy.value) return;
  if (autoUpdateEnabled.value) {
    autoUpdateEnabled.value = false;
    autoPendingTrackerId.value = null;
    resetFirmwareRunItems();
    firmwareProgress.value = 0;
    firmwarePhase.value = firmwareFile.value ? "ready" : "idle";
    firmwareStatusMessage.value = t("flashing.auto_disabled_status");
    return;
  }
  if (!connectedPortName.value) {
    pushLog(t("flashing.require_connection"), "error");
    return;
  }
  if (!firmwareFile.value) {
    pushLog(t("flashing.require_file"), "error");
    return;
  }
  autoUpdateEnabled.value = true;
  armAutoUpdateWaitingState();
}

async function maybeTriggerAutoFirmwareUpdate(): Promise<void> {
  const trackerId = autoPendingTrackerId.value;
  if (
    trackerId === null ||
    firmwareMode.value !== "auto_slot" ||
    !autoUpdateEnabled.value ||
    firmwareBusy.value ||
    !connectedPortName.value ||
    !firmwareFile.value
  ) {
    return;
  }
  const tracker = getTrackerById(trackerId);
  if (!tracker?.inserted) {
    armAutoUpdateWaitingState();
    return;
  }
  if (!tracker.usbPath) {
    return;
  }

  autoPendingTrackerId.value = null;
  const queuedMessage = t("flashing.auto_triggered_status", { id: trackerId, path: tracker.usbPath });
  setFirmwareRunItem(trackerId, "queued", queuedMessage);
  firmwareStatusMessage.value = queuedMessage;

  try {
    await runFirmwareFlashForTracker(trackerId);
  } catch {
  } finally {
    if (firmwareMode.value === "auto_slot" && autoUpdateEnabled.value && !firmwareBusy.value) {
      armAutoUpdateWaitingState();
    }
  }
}

async function startBatchFirmwareFlash(): Promise<void> {
  if (firmwareBusy.value) return;
  if (!connectedPortName.value) {
    pushLog(t("flashing.require_connection"), "error");
    return;
  }
  if (!firmwareFile.value) {
    pushLog(t("flashing.require_file"), "error");
    return;
  }

  autoUpdateEnabled.value = false;
  autoPendingTrackerId.value = null;
  resetFirmwareRunItems();
  firmwareProgress.value = 0;
  firmwarePhase.value = "ready";

  await refreshTrackerStatus();

  const insertedTrackers = firmwareSlotStatuses.value.filter((tracker) => tracker.inserted).map((tracker) => tracker.id);
  if (!insertedTrackers.length) {
    firmwareStatusMessage.value = t("flashing.batch_no_targets_status");
    return;
  }

  for (const tracker of firmwareSlotStatuses.value) {
    if (tracker.inserted) {
      setFirmwareRunItem(tracker.id, "queued", t("flashing.batch_queued_status", { id: tracker.id }));
    } else {
      setFirmwareRunItem(tracker.id, "skipped", t("flashing.batch_skipped_status", { id: tracker.id }));
    }
  }

  for (const trackerId of insertedTrackers) {
    const queueMessage = t("flashing.batch_running_status", { id: trackerId });
    firmwareStatusMessage.value = queueMessage;
    setFirmwareRunItem(trackerId, "queued", queueMessage);
    try {
      await runFirmwareFlashForTracker(trackerId);
    } catch (error) {
      const message = t("flashing.batch_stopped_status", {
        id: trackerId,
        msg: getErrorMessage(error),
      });
      firmwareStatusMessage.value = message;
      pushLog(message, "error");
      return;
    }
  }

  firmwarePhase.value = "success";
  firmwareProgress.value = 100;
  firmwareStatusMessage.value = t("flashing.batch_completed_status");
  pushLog(firmwareStatusMessage.value, "success");
}

async function runSingleAction(action: string, trackerId: number): Promise<void> {
  loading.value = true;
  startOverlayTimer(action);
  try {
    const ack = await invoke<AckResponse>("control_tracker", { action, trackerId });
    if (ack.success) {
      pushLog(t('notifications.action_success', { cmd: ack.cmd, id: trackerId }), 'success');
      await refreshTrackerStatus();
    } else {
      pushLog(t('notifications.action_failed', { msg: ack.msg ?? ack.cmd }), 'error');
    }
  } catch (error) {
    pushLog(t('notifications.action_failed', { msg: getErrorMessage(error) }), 'error');
  } finally {
    loading.value = false;
    stopOverlayTimer();
  }
}

async function runAllAction(action: string): Promise<void> {
  loading.value = true;
  startOverlayTimer(action);
  try {
    const ack = await invoke<AckResponse>("control_all", { action });
    if (ack.success) {
      pushLog(t('notifications.action_all_success', { cmd: ack.cmd }), 'success');
      await refreshTrackerStatus();
    } else {
      pushLog(t('notifications.action_failed', { msg: ack.msg ?? ack.cmd }), 'error');
    }
  } catch (error) {
    pushLog(t('notifications.action_failed', { msg: getErrorMessage(error) }), 'error');
  } finally {
    loading.value = false;
    stopOverlayTimer();
  }
}

async function toggleLed(): Promise<void> {
  loading.value = true;
  try {
    const nextValue = !ledEnabled.value;
    const ack = await invoke<AckResponse>("set_dock_led", { enabled: nextValue });
    if (ack.success) {
      ledEnabled.value = nextValue;
      pushLog(t('notifications.led_success', { status: nextValue ? t('common.on') : t('common.off') }), 'success');
    } else {
      pushLog(t('notifications.led_failed', { msg: ack.msg ?? ack.cmd }), 'error');
    }
  } catch (error) {
    pushLog(t('notifications.led_failed', { msg: getErrorMessage(error) }), 'error');
  } finally {
    loading.value = false;
  }
}

async function setBlMode(mode: number): Promise<void> {
  loading.value = true;
  try {
    const ack = await invoke<AckResponse>("set_bl_mode", { mode });
    if (ack.success) {
      await refreshBlMode();
      pushLog(
        t('notifications.bl_mode_set_success', { mode: blMode.value ?? mode, name: blModeName.value || "-" }),
        'success',
      );
    } else {
      pushLog(t('notifications.bl_mode_set_failed', { msg: ack.msg ?? ack.cmd }), 'error');
    }
  } catch (error) {
    pushLog(t('notifications.bl_mode_set_failed', { msg: getErrorMessage(error) }), 'error');
  } finally {
    loading.value = false;
  }
}

async function setAutoSleep(enabled: boolean): Promise<void> {
  loading.value = true;
  try {
    const ack = await invoke<AckResponse>("set_auto_sleep", { enabled });
    if (ack.success) {
      autoSleepEnabled.value = enabled;
      pushLog(
        t('notifications.auto_sleep_set_success', { status: enabled ? t('common.on') : t('common.off') }),
        'success',
      );
    } else {
      pushLog(t('notifications.auto_sleep_set_failed', { msg: ack.msg ?? ack.cmd }), 'error');
    }
  } catch (error) {
    pushLog(t('notifications.auto_sleep_set_failed', { msg: getErrorMessage(error) }), 'error');
  } finally {
    loading.value = false;
  }
}

const openDebug = async () => {
  await invoke("open_debug_window");
};

function buildWindowDockingConfig(settings: SystemSettings = systemSettings.value): WindowDockingConfig {
  return {
    autoDockOnStartup: settings.autoDockOnStartup,
    dockAlwaysOnTop: settings.dockAlwaysOnTop,
    followSlimeVrWindow: settings.followSlimeVrWindow,
    snapStyleApproximation: settings.snapStyleApproximation,
  };
}

async function syncWindowDockingConfig(settings: SystemSettings = systemSettings.value): Promise<void> {
  await invoke("sync_window_docking_config", {
    config: buildWindowDockingConfig(settings),
  });
  await syncWindowAlwaysOnTopState();
}

async function syncWindowAlwaysOnTopState(): Promise<void> {
  try {
    windowAlwaysOnTop.value = await appWindow.isAlwaysOnTop();
  } catch (error) {
    console.error("[window] failed to read always-on-top state:", error);
  }
}

async function dockWithSlimeVr(options: { announce?: boolean; silentMissing?: boolean } = {}): Promise<void> {
  if (windowDockingBusy.value) return;
  windowDockingBusy.value = true;
  try {
    const result = await invoke<DockWindowsResult>("dock_with_slimevr");
    if (!options.announce) {
      return;
    }
    if (result.slimevrFound && result.slimevrDocked) {
      pushLog(t("notifications.window_docking_success"), "success");
    } else if (!options.silentMissing) {
      pushLog(t("notifications.window_docking_partial"), "info");
    }
  } catch (error) {
    if (options.announce) {
      pushLog(t("notifications.window_docking_failed", { msg: getErrorMessage(error) }), "error");
    } else {
      console.error("[window_docking] startup docking failed:", error);
    }
  } finally {
    await syncWindowAlwaysOnTopState();
    windowDockingBusy.value = false;
  }
}

async function toggleWindowAlwaysOnTop(): Promise<void> {
  const next = !windowAlwaysOnTop.value;
  try {
    await appWindow.setAlwaysOnTop(next);
    windowAlwaysOnTop.value = next;
  } catch (error) {
    console.error("[window] setAlwaysOnTop failed:", error);
  }
}

async function persistSystemSettings(): Promise<void> {
  try {
    await invoke("save_system_settings", {
      settings: systemSettings.value,
    });
  } catch (error) {
    pushLog(t("settings.save_failed", { msg: getErrorMessage(error) }), "error");
  }
}

async function setLanguagePreference(pref: LanguagePreference): Promise<void> {
  systemSettings.value = {
    ...systemSettings.value,
    languagePreference: pref,
  };
  locale.value = resolveLocaleFromPreference(pref);
  await persistSystemSettings();
}

async function setDebugEnabled(enabled: boolean): Promise<void> {
  systemSettings.value = {
    ...systemSettings.value,
    debugEnabled: enabled,
  };
  await persistSystemSettings();
}

async function setAutoCheckUpdate(enabled: boolean): Promise<void> {
  systemSettings.value = {
    ...systemSettings.value,
    autoCheckUpdate: enabled,
  };
  await persistSystemSettings();
}

async function updateWindowDockingSettings(
  patch: Partial<WindowDockingConfig>,
): Promise<void> {
  systemSettings.value = {
    ...systemSettings.value,
    ...patch,
  };
  await persistSystemSettings();
  try {
    await syncWindowDockingConfig();
  } catch (error) {
    pushLog(t("notifications.window_docking_config_failed", { msg: getErrorMessage(error) }), "error");
  }
}

async function setAutoDockOnStartup(enabled: boolean): Promise<void> {
  await updateWindowDockingSettings({ autoDockOnStartup: enabled });
}

async function setDockAlwaysOnTop(enabled: boolean): Promise<void> {
  await updateWindowDockingSettings({ dockAlwaysOnTop: enabled });
}

async function setFollowSlimeVrWindow(enabled: boolean): Promise<void> {
  await updateWindowDockingSettings({ followSlimeVrWindow: enabled });
}

async function setSnapStyleApproximation(enabled: boolean): Promise<void> {
  await updateWindowDockingSettings({ snapStyleApproximation: enabled });
}

async function checkForAppUpdate(options: { silentNoUpdate?: boolean } = {}): Promise<void> {
  if (appUpdateChecking.value || appUpdateInstalling.value) return;
  appUpdateChecking.value = true;
  setAppUpdateStatus("settings.update_status_checking");
  try {
    const update = await check();
    if (!update) {
      pendingAppUpdate.value = null;
      appUpdateAvailable.value = false;
      appUpdateVersion.value = null;
      setAppUpdateStatus("settings.update_status_latest");
      if (!options.silentNoUpdate) {
        pushLog(t("settings.update_status_latest"), "info");
      }
      return;
    }
    pendingAppUpdate.value = markRaw(update);
    appUpdateAvailable.value = true;
    appUpdateVersion.value = update.version;
    setAppUpdateStatus("settings.update_status_available", {
      version: update.version,
    });
    pushLog(t("settings.update_status_available", { version: update.version }), "success");
  } catch (error) {
    pendingAppUpdate.value = null;
    appUpdateAvailable.value = false;
    appUpdateVersion.value = null;
    const msg = getErrorMessage(error);
    setAppUpdateStatus("settings.update_status_failed", { msg });
    pushLog(t("settings.update_status_failed", { msg }), "error");
  } finally {
    appUpdateChecking.value = false;
  }
}

async function installAppUpdate(): Promise<void> {
  if (appUpdateInstalling.value || appUpdateChecking.value) return;
  if (!pendingAppUpdate.value) {
    await checkForAppUpdate({ silentNoUpdate: true });
  }
  if (!pendingAppUpdate.value) return;
  appUpdateInstalling.value = true;
  setAppUpdateStatus("settings.update_status_installing");
  try {
    await pendingAppUpdate.value.downloadAndInstall();
    setAppUpdateStatus("settings.update_status_ready_restart");
    pushLog(t("settings.update_status_ready_restart"), "success");
  } catch (error) {
    const msg = getErrorMessage(error);
    setAppUpdateStatus("settings.update_status_install_failed", { msg });
    pushLog(t("settings.update_status_install_failed", { msg }), "error");
  } finally {
    appUpdateInstalling.value = false;
  }
}

// --- 生命周期 ---
onMounted(async () => {
  // 须尽早调度：若放在 refreshDocks 等 await 之后，底座扫描慢时用户会感觉「启动从未检测更新」
  if (!isDebugWindow.value && systemSettings.value.autoCheckUpdate && import.meta.env.PROD) {
    window.setTimeout(() => {
      void checkForAppUpdate({ silentNoUpdate: true });
    }, 1500);
  }

  await syncWindowAlwaysOnTopState();
  unlistenDock = await listen<any>("dock-event", (event) => {
    if (isDebugWindow.value) return; 
    const data = event.payload;
    if (data.type === "event") {
      if (data.event === "inserted" || data.event === "removed") {
        const id = data.id;
        const inserted = data.event === "inserted";
        markTrackerEvent(id, inserted);
        trackers.value = trackers.value.map(t =>
          t.id === id
            ? {
                ...t,
                inserted,
                usbPath: inserted ? t.usbPath : undefined,
                trackerVersion: inserted ? t.trackerVersion : undefined,
              }
            : t,
        );
        if (inserted) {
          if (
            firmwareMode.value === "auto_slot" &&
            autoUpdateEnabled.value &&
            id === firmwareTrackerId.value &&
            !firmwareBusy.value &&
            firmwareFile.value
          ) {
            autoPendingTrackerId.value = id;
            const message = t("flashing.auto_waiting_topology_status", { id });
            setFirmwareRunItem(id, "queued", message);
            firmwareStatusMessage.value = message;
            pushLog(message, "info");
          }
          void scanUsbTopology();
        } else if (
          firmwareMode.value === "auto_slot" &&
          autoUpdateEnabled.value &&
          id === firmwareTrackerId.value &&
          !firmwareBusy.value
        ) {
          armAutoUpdateWaitingState();
        }
        const eventKey = inserted ? 'notifications.event_inserted' : 'notifications.event_removed';
        pushLog(t(eventKey, { id }), 'info');
      } else if (data.event === "boot") {
        pushLog(t('notifications.event_boot', { project: data.project, version: data.version }), 'success');
        void refreshTrackerStatus();
      }
    } else if (data.type === "status") {
      trackers.value = normalizeTrackers(data.trackers ?? []);
      if (typeof data.led === "boolean") ledEnabled.value = data.led;
      if (typeof data.bl_mode === "number") blMode.value = data.bl_mode;
      if (typeof data.auto_sleep === "boolean") autoSleepEnabled.value = data.auto_sleep;
    } else if (data.type === "bl_mode") {
      if (typeof data.mode === "number") blMode.value = data.mode;
      if (typeof data.name === "string") blModeName.value = data.name;
    } else if (data.type === "auto_sleep") {
      if (typeof data.enabled === "boolean") autoSleepEnabled.value = data.enabled;
    }
  });
  unlistenFirmware = await listen<FirmwareProgressEvent>("firmware-progress", (event) => {
    if (isDebugWindow.value) return;
    const data = event.payload;
    const activeTrackerId = activeFirmwareTrackerId.value ?? firmwareTrackerId.value;
    if (data.trackerId !== activeTrackerId) return;
    firmwarePhase.value = data.phase;
    firmwareProgress.value = data.progress;
    firmwareStatusMessage.value = resolveMessage(data.message);
    setFirmwareRunItem(data.trackerId, "running", firmwareStatusMessage.value);
  });

  await refreshDocks();
  await loadConnectedPort();
  if (connectedPortName.value) {
    await refreshDockInfo();
    await refreshTrackerStatus();
    await refreshBlMode();
    await refreshAutoSleep();
  }
  if (!isDebugWindow.value) {
    connectionMonitorTimer = window.setInterval(() => {
      void checkDockConnectionHealth();
    }, 1500);
    if (systemSettings.value.autoDockOnStartup) {
      window.setTimeout(() => {
        void dockWithSlimeVr({ announce: false, silentMissing: true });
      }, 250);
    }
  }
});

onUnmounted(() => {
  if (unlistenDock) unlistenDock();
  if (unlistenFirmware) unlistenFirmware();
  if (connectionMonitorTimer) {
    clearInterval(connectionMonitorTimer);
    connectionMonitorTimer = null;
  }
});
</script>

<template>
  <DebugConsole v-if="isDebugWindow" />

  <main v-else class="page">
    <WindowTitleBar
      :is-always-on-top="windowAlwaysOnTop"
      :docking-busy="windowDockingBusy"
      @toggle-always-on-top="toggleWindowAlwaysOnTop"
      @redock-windows="dockWithSlimeVr({ announce: true })"
    />

    <!-- 全屏遮罩层 -->
    <Teleport to="body">
      <div v-if="showOverlay" class="loading-overlay">
        <div class="loading-content">
          <BaseSpinner />
          <p>{{ t('common.processing') }}</p>
          <div class="progress-info">
            {{ t('common.execution_time', { elapsed: elapsedTime.toFixed(1), estimated: estimatedTime.toFixed(1) }) }}
          </div>
          <div class="progress-bar-container">
            <div class="progress-bar" :style="{ width: Math.min((elapsedTime / estimatedTime) * 100, 100) + '%' }"></div>
          </div>
        </div>
      </div>
    </Teleport>

    <div class="main-content">
      <Home 
        v-if="currentView === 'home'"
        :docks="docks"
        :connected-port-name="connectedPortName"
        :dock-info="dockInfo"
        :trackers="trackers"
        :led-enabled="ledEnabled"
        :loading="uiBusy"
        :bl-mode="blMode"
        :bl-mode-name="blModeName"
        :auto-sleep-enabled="autoSleepEnabled"
        :bl-mode-options="blModeOptions"
        @refresh-docks="refreshDocks"
        @connect-dock="connectDock"
        @disconnect-dock="disconnectDock"
        @run-single-action="runSingleAction"
        @run-all-action="runAllAction"
        @toggle-led="toggleLed"
        @refresh-status="refreshTrackerStatus"
        @set-bl-mode="setBlMode"
        @set-auto-sleep="setAutoSleep"
      />
      <TrackerFlashing
        v-else-if="currentView === 'flashing'"
        :connected-port-name="connectedPortName"
        :docks="docks"
        :dock-info="dockInfo"
        :trackers="trackers"
        :loading="uiBusy"
        :busy="firmwareBusy"
        :mode="firmwareMode"
        :auto-update-enabled="autoUpdateEnabled"
        :selected-tracker-id="firmwareTrackerId"
        :selected-tracker-inserted="selectedFirmwareTracker?.inserted ?? false"
        :selected-tracker-usb-path="selectedFirmwareTracker?.usbPath ?? ''"
        :file-name="firmwareFile?.name ?? ''"
        :file-size="firmwareFile?.size ?? 0"
        :active-tracker-id="activeFirmwareTrackerId ?? 0"
        :phase="firmwarePhase"
        :progress="firmwareProgress"
        :status-message="firmwareStatusMessage"
        :slot-statuses="firmwareSlotStatuses"
        @set-mode="setFirmwareMode"
        @set-tracker-id="setFirmwareTrackerId"
        @select-file="selectFirmwareFile"
        @start-flash="startFirmwareFlash"
        @toggle-auto-update="toggleAutoUpdate"
        @start-batch-flash="startBatchFirmwareFlash"
        @refresh-status="refreshTrackerStatus"
        @refresh-docks="refreshDocks"
        @connect-dock="connectDock"
        @disconnect-dock="disconnectDock"
      />
      <Settings
        v-else-if="currentView === 'settings'"
        :language-preference="systemSettings.languagePreference"
        :debug-enabled="systemSettings.debugEnabled"
        :auto-check-update="systemSettings.autoCheckUpdate"
        :auto-dock-on-startup="systemSettings.autoDockOnStartup"
        :dock-always-on-top="systemSettings.dockAlwaysOnTop"
        :follow-slime-vr-window="systemSettings.followSlimeVrWindow"
        :snap-style-approximation="systemSettings.snapStyleApproximation"
        :update-checking="appUpdateChecking"
        :update-installing="appUpdateInstalling"
        :update-available="appUpdateAvailable"
        :update-version="appUpdateVersion"
        :update-status-text="appUpdateStatusText"
        :window-docking-busy="windowDockingBusy"
        @update:language-preference="setLanguagePreference"
        @update:debug-enabled="setDebugEnabled"
        @update:auto-check-update="setAutoCheckUpdate"
        @update:auto-dock-on-startup="setAutoDockOnStartup"
        @update:dock-always-on-top="setDockAlwaysOnTop"
        @update:follow-slime-vr-window="setFollowSlimeVrWindow"
        @update:snap-style-approximation="setSnapStyleApproximation"
        @check-update="checkForAppUpdate"
        @install-update="installAppUpdate"
        @open-debug="openDebug"
        @redock-windows="dockWithSlimeVr({ announce: true })"
      />
    </div>

    <!-- 底部任务栏 -->
    <nav class="taskbar">
      <button 
        class="task-item" 
        :class="{ active: currentView === 'home' }"
        @click="currentView = 'home'"
      >
        <span class="task-label">{{ t('nav.home') }}</span>
      </button>
      <button 
        class="task-item" 
        :class="{ active: currentView === 'flashing' }"
        @click="currentView = 'flashing'"
      >
        <span class="task-label">{{ t('nav.flashing') }}</span>
      </button>
      <button 
        class="task-item" 
        :class="{ active: currentView === 'settings' }"
        @click="currentView = 'settings'"
      >
        <span class="task-label">{{ t('nav.settings') }}</span>
      </button>
    </nav>

    <NotificationManager :notifications="notifications" />
  </main>
</template>

<style scoped>
.page {
  min-height: var(--window-min-height);
  height: 100%;
  width: var(--window-width);
  margin: 0 auto;
  background: var(--color-bg-page);
  color: var(--color-text-main);
  box-shadow: 0 0 20px rgba(0, 0, 0, 0.1);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.main-content {
  flex: 1;
  padding: var(--spacing-md);
  overflow-y: auto;
  scrollbar-gutter: stable;
  margin-bottom: 2px; /* For the taskbar separation */
}

/* 任务栏样式 */
.taskbar {
  display: flex;
  background: var(--color-bg-header);
  border-top: var(--border-width) solid var(--color-secondary);
  height: 48px; /* Reduced height for text-only */
  padding: 0;
}

.task-item {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--color-text-light);
  cursor: pointer;
  transition: all 0.2s;
  padding: 0;
  border-bottom: 4px solid transparent;
  height: 100%;
}

.task-item:hover {
  background: var(--color-secondary-hover);
  color: var(--color-primary);
}

.task-item.active {
  background: var(--color-bg-white);
  color: var(--color-primary);
  border-bottom-color: var(--color-primary);
}

.task-label {
  font-size: 14px; /* Slightly larger text */
  font-weight: bold;
}

/* 其他样式保留 */
.loading-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background: rgba(0, 0, 0, 0.4);
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 9999;
  backdrop-filter: blur(2px);
}

.loading-content {
  background: var(--color-bg-white);
  padding: 30px 50px;
  border: var(--border-width) solid var(--color-primary);
  box-shadow: var(--box-shadow-heavy);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--spacing-lg);
}

.loading-content p {
  margin: 0;
  font-weight: bold;
  color: var(--color-primary);
  font-size: 16px;
}

.progress-info {
  font-family: var(--font-family-mono);
  font-size: 14px;
  color: var(--color-text-secondary);
}

.progress-bar-container {
  width: 240px;
  height: 8px;
  background: var(--color-progress-track);
  border: var(--border-width-subtle) solid var(--color-primary);
  border-radius: 0;
}

.progress-bar {
  height: 100%;
  background: var(--color-primary);
  transition: width 0.1s linear;
  border-radius: 0;
}
</style>
