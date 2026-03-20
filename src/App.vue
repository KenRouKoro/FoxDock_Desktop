<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useI18n } from "vue-i18n";

// 导入组件
import Home from "./views/Home.vue";
import TrackerFlashing from "./views/TrackerFlashing.vue";
import Settings from "./views/Settings.vue";
import NotificationManager from "./components/NotificationManager.vue";
import BaseSpinner from "./components/ui/BaseSpinner.vue";
import WindowTitleBar from "./components/ui/WindowTitleBar.vue";
import DebugConsole from "./components/DebugConsole.vue";

const { t, locale } = useI18n();

// --- 类型定义 ---
type DockPort = {
  portName: string;
  displayName: string;
  serialNumber: string | null;
};

type DockInfo = {
  project: string;
  version: string;
  mcu: string;
  extra?: Record<string, unknown>;
};

type TrackerStatus = {
  id: number;
  inserted: boolean;
  usbPath?: string;
};

type UsbTopologyResult = {
  id: number;
  inserted: boolean;
  usb_path?: string;
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
  trackers: TrackerStatus[];
};

type BlModeResponse = {
  mode: number;
  name?: string;
};

type AutoSleepResponse = {
  enabled: boolean;
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
const blModeOptions = computed(() => [
  { mode: 0, label: t("tracker_control.bl_mode_option_0") },
  { mode: 1, label: t("tracker_control.bl_mode_option_1") },
  { mode: 2, label: t("tracker_control.bl_mode_option_2") },
]);

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

function resetConnectedState(): void {
  connectedPortName.value = "";
  dockInfo.value = null;
  trackers.value = normalizeTrackers([]);
  ledEnabled.value = false;
  blMode.value = null;
  blModeName.value = "";
  autoSleepEnabled.value = false;
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

function normalizeTrackers(current: TrackerStatus[]): TrackerStatus[] {
  const map = new Map(current.map((item) => [item.id, item]));
  return Array.from({ length: 10 }, (_, index) => {
    const id = index + 1;
    const existing = trackers.value.find(t => t.id === id);
    const newData = map.get(id);
    return {
      id,
      inserted: newData?.inserted ?? false,
      usbPath: newData?.usbPath ?? existing?.usbPath
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
  if (!connectedPortName.value || loading.value) return;
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
          usbPath: usbInfo?.usb_path ?? t.usbPath
        };
      });
    } catch (error) {
      console.error("[USB] Topology scan failed:", error);
    } finally {
      scanTimeout = null;
    }
  }, 2000);
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

const toggleLocale = () => {
  locale.value = locale.value === 'zh' ? 'en' : 'zh';
};

// --- 生命周期 ---
onMounted(async () => {
  unlistenDock = await listen<any>("dock-event", (event) => {
    if (isDebugWindow.value) return; 
    const data = event.payload;
    if (data.type === "event") {
      if (data.event === "inserted" || data.event === "removed") {
        const id = data.id;
        const inserted = data.event === "inserted";
        markTrackerEvent(id, inserted);
        trackers.value = trackers.value.map(t => t.id === id ? { ...t, inserted, usbPath: inserted ? t.usbPath : undefined } : t);
        if (inserted) {
          void scanUsbTopology();
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
  }
});

onUnmounted(() => {
  if (unlistenDock) unlistenDock();
  if (connectionMonitorTimer) {
    clearInterval(connectionMonitorTimer);
    connectionMonitorTimer = null;
  }
});
</script>

<template>
  <DebugConsole v-if="isDebugWindow" />

  <main v-else class="page">
    <WindowTitleBar />

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
        :loading="loading"
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
        @toggle-locale="toggleLocale"
        @open-debug="openDebug"
      />
      <TrackerFlashing v-else-if="currentView === 'flashing'" />
      <Settings v-else-if="currentView === 'settings'" />
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
  background: #f0f0f0;
  border: 1px solid var(--color-primary);
  border-radius: 0;
}

.progress-bar {
  height: 100%;
  background: var(--color-primary);
  transition: width 0.1s linear;
  border-radius: 0;
}
</style>
