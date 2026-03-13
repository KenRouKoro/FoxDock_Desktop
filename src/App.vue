<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useI18n } from "vue-i18n";

// 导入组件
import DebugConsole from "./components/DebugConsole.vue";
import ConnectionPanel from "./components/ConnectionPanel.vue";
import TrackerStatusComponent from "./components/TrackerStatus.vue";
import TrackerControl from "./components/TrackerControl.vue";
import NotificationManager from "./components/NotificationManager.vue";

// 导入资源
import logoUrl from "./assets/FoxApplication.png";

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

interface Notification {
  id: number;
  message: string;
  type: 'info' | 'success' | 'error';
  timestamp: number;
}

// --- 路由模拟 ---
const searchParams = new URLSearchParams(window.location.search);
const isDebugWindow = ref(searchParams.get("debug") === "true");

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
}

// --- 核心业务逻辑 ---
function getErrorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
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
    const result = await invoke<TrackerStatus[]>("get_dock_status");
    trackers.value = normalizeTrackers(result);
    // 在获取状态后，尝试进行一次 USB 拓扑扫描
    console.log("[App] Triggering scan after status refresh");
    await scanUsbTopology();
  } catch (error) {
    pushLog(t('notifications.tracker_status_failed', { msg: getErrorMessage(error) }), 'error');
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
    console.log(`[USB] Scan blocked due to unstable events on slot ${id}`);
    if (scanTimeout) {
      clearTimeout(scanTimeout);
      scanTimeout = null;
    }
  }
  trackerLastEvent.set(id, { inserted, ts: now });
}

async function scanUsbTopology(): Promise<void> {
  if (!connectedPortName.value) return;
  if (Date.now() < topologyScanBlockedUntil) {
    console.log("[USB] Scan skipped due to unstable insert/remove events.");
    return;
  }
  
  if (scanTimeout) {
    console.log("[USB] Resetting scan timeout...");
    clearTimeout(scanTimeout);
  }

  console.log("[USB] Scheduling scan in 2s...");
  scanTimeout = window.setTimeout(async () => {
    try {
      if (Date.now() < topologyScanBlockedUntil) {
        console.log("[USB] Scan canceled before execution due to unstable events.");
        return;
      }
      console.log("[USB] Starting topology scan...");
      const usbResults = await invoke<UsbTopologyResult[]>("scan_usb_topology");
      console.log("[USB] Scan results:", usbResults);
      
      // 合并 USB 路径信息到当前的 trackers 状态中
      trackers.value = trackers.value.map(t => {
        const usbInfo = usbResults.find(u => u.id === t.id);
        if (usbInfo) {
          console.log(`[USB] Slot ${t.id} matched to path: ${usbInfo.usb_path}`);
        }
        return {
          ...t,
          usbPath: usbInfo?.usb_path ?? t.usbPath
        };
      });
      console.log("[USB] Topology update complete.");
    } catch (error) {
      console.error("[USB] Topology scan failed:", error);
    } finally {
      scanTimeout = null;
    }
  }, 2000); // 延迟 2s 以等待 USB 握手
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

const openDebug = async () => {
  await invoke("open_debug_window");
};

const toggleLocale = () => {
  locale.value = locale.value === 'zh' ? 'en' : 'zh';
};

// --- 生命周期 ---
onMounted(async () => {
  // 1. 底座主动事件监听
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
      trackers.value = normalizeTrackers(data.trackers);
    }
  });

  await refreshDocks();
  await loadConnectedPort();
  if (connectedPortName.value) {
    await refreshDockInfo();
    await refreshTrackerStatus();
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
    <!-- 全屏遮罩层 -->
    <Teleport to="body">
      <div v-if="showOverlay" class="loading-overlay">
        <div class="loading-content">
          <div class="spinner"></div>
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

    <header class="header">
      <div class="header-bar">
        <div class="header-title-area">
          <img :src="logoUrl" class="logo" alt="FoxDock Logo" />
          <h1>{{ t('app.title') }}</h1>
        </div>
        <div class="header-actions">
          <button class="lang-btn" @click="toggleLocale">{{ locale === 'zh' ? 'English' : '中文' }}</button>
          <button class="debug-btn" @click="openDebug">{{ t('app.debug_btn') }}</button>
        </div>
      </div>
      <p>{{ t('app.subtitle') }}</p>
    </header>

    <ConnectionPanel 
      :docks="docks"
      :connected-port-name="connectedPortName"
      :dock-info="dockInfo"
      :loading="loading"
      @refresh="refreshDocks"
      @connect="connectDock"
      @disconnect="disconnectDock"
    />

    <TrackerStatusComponent 
      :trackers="trackers" 
      :disabled="loading || !connectedPortName"
      @run-single-action="runSingleAction"
    />

    <TrackerControl 
      :connected-port-name="connectedPortName"
      :loading="loading"
      :led-enabled="ledEnabled"
      @run-single-action="runSingleAction"
      @run-all-action="runAllAction"
      @toggle-led="toggleLed"
      @refresh-status="refreshTrackerStatus"
    />

    <NotificationManager :notifications="notifications" />
  </main>
</template>

<style scoped>
.page {
  min-height: 100vh;
  margin: 0;
  padding: 20px;
  background: #e8f2ff;
  color: #12304f;
  font-family: "Segoe UI", Arial, sans-serif;
}

.header {
  margin-bottom: 16px;
  border: 2px solid #59a9ff;
  background: #d7ebff;
  padding: 12px 16px;
}

.header h1 {
  margin: 0 0 6px;
  font-size: 24px;
}

.header p {
  margin: 0;
  color: #2f5d8f;
}

.header-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.header-title-area {
  display: flex;
  align-items: center;
  gap: 12px;
}

.logo {
  height: 40px;
  width: auto;
}

.header-actions {
  display: flex;
  gap: 8px;
}

.debug-btn, .lang-btn {
  background: #0078d4;
  color: white;
  border: none;
  padding: 6px 12px;
  cursor: pointer;
  font-size: 12px;
}

.lang-btn {
  background: #ffffff;
  color: #0078d4;
  border: 2px solid #0078d4;
}

.debug-btn:hover {
  background: #106ebe;
}

.lang-btn:hover {
  background: #f3f9ff;
}

/* 全屏遮罩样式 */
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
  background: white;
  padding: 30px 50px;
  border: 2px solid #0078d4;
  box-shadow: 8px 8px 0 rgba(0, 0, 0, 0.2);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
}

.loading-content p {
  margin: 0;
  font-weight: bold;
  color: #0078d4;
  font-size: 16px;
}

.progress-info {
  font-family: "Courier New", Courier, monospace;
  font-size: 14px;
  color: #184470;
}

.progress-bar-container {
  width: 240px;
  height: 8px;
  background: #f0f0f0;
  border: 1px solid #0078d4;
}

.progress-bar {
  height: 100%;
  background: #0078d4;
  transition: width 0.1s linear;
}

.spinner {
  width: 40px;
  height: 40px;
  border: 4px solid #f3f3f3;
  border-top: 4px solid #0078d4;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}
</style>
