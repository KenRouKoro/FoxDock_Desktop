<script setup lang="ts">
import { ref, onMounted, onUnmounted, onBeforeUnmount, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// --- 路由模拟 ---
const searchParams = new URLSearchParams(window.location.search);
const isDebugWindow = ref(searchParams.get("debug") === "true");

// --- Debug 页面数据 ---
interface DebugLog {
  direction: string;
  content: string;
  timestamp: string;
}

const debugLogs = ref<DebugLog[]>([]);
const logContainer = ref<HTMLElement | null>(null);
let unlistenDebug: (() => void) | null = null;
let unlistenDock: (() => void) | null = null;

onMounted(async () => {
  // 1. 串口调试日志监听
  unlistenDebug = await listen<DebugLog>("serial-debug-log", (event) => {
    if (isDebugWindow.value) {
      const newLog = event.payload;
      const lastLog = debugLogs.value[debugLogs.value.length - 1];

      // 合并逻辑：如果方向相同，且距离上一条日志时间很短（例如小于 100ms），则合并显示
      if (lastLog && lastLog.direction === newLog.direction) {
        lastLog.content += newLog.content;
        // 更新时间戳为最新的
        lastLog.timestamp = newLog.timestamp;
      } else {
        debugLogs.value.push(newLog);
      }

      if (debugLogs.value.length > 2000) debugLogs.value.shift();
      
      nextTick(() => {
        if (logContainer.value) {
          logContainer.value.scrollTop = logContainer.value.scrollHeight;
        }
      });
    }
  });

  // 2. 底座主动事件监听 (inserted, removed, boot)
  unlistenDock = await listen<any>("dock-event", (event) => {
    if (isDebugWindow.value) return; 
    
    const data = event.payload;
    if (data.type === "event") {
      if (data.event === "inserted" || data.event === "removed") {
        const id = data.id;
        const inserted = data.event === "inserted";
        trackers.value = trackers.value.map(t => 
          t.id === id ? { ...t, inserted } : t
        );
        pushLog(`[事件] 槽位 #${id} ${inserted ? "已插入" : "已拔出"}`);
      } else if (data.event === "boot") {
        pushLog(`[事件] 底座已重启: ${data.project} v${data.version}`);
        void refreshTrackerStatus();
      }
    } else if (data.type === "status") {
      trackers.value = normalizeTrackers(data.trackers);
    }
  });
});

onUnmounted(() => {
  if (unlistenDebug) unlistenDebug();
  if (unlistenDock) unlistenDock();
});

const clearLogs = () => {
  debugLogs.value = [];
};

const openDebug = async () => {
  await invoke("open_debug_window");
};

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
};

type AckResponse = {
  cmd: string;
  success: boolean;
  msg?: string;
};

const docks = ref<DockPort[]>([]);
const selectedPortName = ref("");
const connectedPortName = ref("");
const dockInfo = ref<DockInfo | null>(null);
const trackers = ref<TrackerStatus[]>(
  Array.from({ length: 10 }, (_, index) => ({ id: index + 1, inserted: false })),
);
const selectedTrackerId = ref(1);
const ledEnabled = ref(false);
const loading = ref(false);
const logs = ref<string[]>([]);

// --- 状态定义 ---
const singleActions = [
  { label: "复位", value: "ret" },
  { label: "Bootloader", value: "bl" },
  { label: "休眠", value: "sleep" },
  { label: "配对", value: "pair" },
];

const allActions = [
  { label: "全部复位", value: "ret_all" },
  { label: "全部 Bootloader", value: "bl_all" },
  { label: "全部休眠", value: "sleep_all" },
  { label: "全部配对", value: "pair_all" },
];

function getErrorMessage(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return "未知错误";
}

function pushLog(message: string): void {
  const time = new Date().toLocaleTimeString();
  logs.value.unshift(`[${time}] ${message}`);
  if (logs.value.length > 12) {
    logs.value = logs.value.slice(0, 12);
  }
}

function normalizeTrackers(current: TrackerStatus[]): TrackerStatus[] {
  const map = new Map(current.map((item) => [item.id, item.inserted]));
  return Array.from({ length: 10 }, (_, index) => ({
    id: index + 1,
    inserted: map.get(index + 1) ?? false,
  }));
}

async function refreshDocks(): Promise<void> {
  loading.value = true;
  try {
    const result = await invoke<DockPort[]>("discover_docks");
    docks.value = result;
    if (!selectedPortName.value && result.length > 0) {
      selectedPortName.value = result[0].portName;
    }
    pushLog(`扫描到 ${result.length} 个可用底座`);
  } catch (error) {
    pushLog(`扫描失败：${getErrorMessage(error)}`);
  } finally {
    loading.value = false;
  }
}

async function loadConnectedPort(): Promise<void> {
  try {
    const current = await invoke<string | null>("get_connected_port");
    connectedPortName.value = current ?? "";
    if (connectedPortName.value) {
      selectedPortName.value = connectedPortName.value;
    }
  } catch (error) {
    pushLog(`读取连接状态失败：${getErrorMessage(error)}`);
  }
}

async function connectDock(): Promise<void> {
  if (!selectedPortName.value) {
    pushLog("请先选择底座端口");
    return;
  }
  loading.value = true;
  try {
    const dock = await invoke<DockPort>("connect_dock", {
      portName: selectedPortName.value,
    });
    connectedPortName.value = dock.portName;
    pushLog(`已连接 ${dock.displayName}`);
    await refreshDockInfo();
    await refreshTrackerStatus();
  } catch (error) {
    pushLog(`连接失败：${getErrorMessage(error)}`);
  } finally {
    loading.value = false;
  }
}

async function disconnectDock(): Promise<void> {
  loading.value = true;
  try {
    await invoke("disconnect_dock");
    connectedPortName.value = "";
    dockInfo.value = null;
    trackers.value = normalizeTrackers([]);
    ledEnabled.value = false;
    pushLog("已断开连接");
  } catch (error) {
    pushLog(`断开失败：${getErrorMessage(error)}`);
  } finally {
    loading.value = false;
  }
}

async function refreshDockInfo(): Promise<void> {
  if (!connectedPortName.value) {
    return;
  }
  try {
    dockInfo.value = await invoke<DockInfo>("get_dock_info");
  } catch (error) {
    pushLog(`读取底座信息失败：${getErrorMessage(error)}`);
  }
}

async function refreshTrackerStatus(): Promise<void> {
  if (!connectedPortName.value) {
    return;
  }
  try {
    const result = await invoke<TrackerStatus[]>("get_dock_status");
    trackers.value = normalizeTrackers(result);
  } catch (error) {
    pushLog(`读取追踪器状态失败：${getErrorMessage(error)}`);
  }
}

async function runSingleAction(action: string): Promise<void> {
  if (!connectedPortName.value) {
    pushLog("请先连接底座");
    return;
  }
  loading.value = true;
  try {
    const ack = await invoke<AckResponse>("control_tracker", {
      action,
      trackerId: selectedTrackerId.value,
    });
    if (ack.success) {
      pushLog(`执行成功：${ack.cmd} #${selectedTrackerId.value}`);
      await refreshTrackerStatus();
    } else {
      pushLog(`执行失败：${ack.msg ?? ack.cmd}`);
    }
  } catch (error) {
    pushLog(`执行失败：${getErrorMessage(error)}`);
  } finally {
    loading.value = false;
  }
}

async function runAllAction(action: string): Promise<void> {
  if (!connectedPortName.value) {
    pushLog("请先连接底座");
    return;
  }
  loading.value = true;
  try {
    const ack = await invoke<AckResponse>("control_all", { action });
    if (ack.success) {
      pushLog(`执行成功：${ack.cmd}`);
      await refreshTrackerStatus();
    } else {
      pushLog(`执行失败：${ack.msg ?? ack.cmd}`);
    }
  } catch (error) {
    pushLog(`执行失败：${getErrorMessage(error)}`);
  } finally {
    loading.value = false;
  }
}

async function toggleLed(): Promise<void> {
  if (!connectedPortName.value) {
    pushLog("请先连接底座");
    return;
  }
  loading.value = true;
  try {
    const nextValue = !ledEnabled.value;
    const ack = await invoke<AckResponse>("set_dock_led", { enabled: nextValue });
    if (ack.success) {
      ledEnabled.value = nextValue;
      pushLog(`底座 LED 已${nextValue ? "开启" : "关闭"}`);
    } else {
      pushLog(`LED 控制失败：${ack.msg ?? ack.cmd}`);
    }
  } catch (error) {
    pushLog(`LED 控制失败：${getErrorMessage(error)}`);
  } finally {
    loading.value = false;
  }
}

onMounted(async () => {
  await refreshDocks();
  await loadConnectedPort();
  if (connectedPortName.value) {
    await refreshDockInfo();
    await refreshTrackerStatus();
  }
});

onBeforeUnmount(() => {
  // 保持为空或移除，因为不再需要 statusTimer
});
</script>

<template>
  <main v-if="isDebugWindow" class="debug-container">
    <header class="debug-header">
      <span class="title">Serial Debug Console</span>
      <button class="clear-btn" @click="clearLogs">Clear Logs</button>
    </header>
    <div ref="logContainer" class="log-list">
      <div v-for="(log, index) in debugLogs" :key="index" class="log-item" :class="log.direction.toLowerCase()">
        <span class="ts">[{{ log.timestamp }}]</span>
        <span class="dir">{{ log.direction }}</span>
        <pre class="content">{{ log.content }}</pre>
      </div>
    </div>
  </main>

  <main v-else class="page">
    <header class="header">
      <div class="header-bar">
        <h1>FoxDock 桌面控制台</h1>
        <button class="debug-btn" @click="openDebug">调试</button>
      </div>
      <p>10 槽位追踪器底座管理</p>
    </header>

    <section class="panel">
      <h2>连接管理</h2>
      <div class="row">
        <select v-model="selectedPortName" :disabled="loading">
          <option value="">请选择底座端口</option>
          <option v-for="dock in docks" :key="dock.portName" :value="dock.portName">
            {{ dock.displayName }}
          </option>
        </select>
        <button :disabled="loading" @click="refreshDocks">刷新</button>
        <button :disabled="loading || !selectedPortName" @click="connectDock">连接</button>
        <button :disabled="loading || !connectedPortName" @click="disconnectDock">断开</button>
      </div>
      <p class="status">
        当前连接：
        <span v-if="connectedPortName">{{ connectedPortName }}</span>
        <span v-else>未连接</span>
      </p>
      <div class="info-grid">
        <div>项目：{{ dockInfo?.project ?? "-" }}</div>
        <div>版本：{{ dockInfo?.version ?? "-" }}</div>
        <div>MCU：{{ dockInfo?.mcu ?? "-" }}</div>
      </div>
    </section>

    <section class="panel">
      <h2>追踪器插入状态</h2>
      <div class="tracker-column">
        <div v-for="item in trackers" :key="item.id" class="tracker-cell" :class="{ inserted: item.inserted }">
          <span>槽位 {{ item.id }}</span>
          <span>{{ item.inserted ? "已插入" : "未插入" }}</span>
        </div>
      </div>
    </section>

    <section class="panel">
      <h2>追踪器控制</h2>
      <div class="row">
        <label for="trackerId">槽位</label>
        <select id="trackerId" v-model.number="selectedTrackerId" :disabled="loading">
          <option v-for="id in 10" :key="id" :value="id">{{ id }}</option>
        </select>
      </div>
      <div class="button-grid">
        <button
          v-for="item in singleActions"
          :key="item.value"
          :disabled="loading || !connectedPortName"
          @click="runSingleAction(item.value)"
        >
          {{ item.label }}
        </button>
      </div>
      <div class="button-grid">
        <button
          v-for="item in allActions"
          :key="item.value"
          :disabled="loading || !connectedPortName"
          @click="runAllAction(item.value)"
        >
          {{ item.label }}
        </button>
      </div>
      <div class="row">
        <button :disabled="loading || !connectedPortName" @click="toggleLed">
          底座 LED：{{ ledEnabled ? "开" : "关" }}
        </button>
        <button :disabled="loading || !connectedPortName" @click="refreshTrackerStatus">刷新状态</button>
      </div>
    </section>

    <section class="panel">
      <h2>运行日志</h2>
      <div class="log-list">
        <div v-for="line in logs" :key="line" class="log-line">{{ line }}</div>
      </div>
    </section>
  </main>
</template>

<style scoped>
/* --- Metro UI & Debug Window Styles --- */
.debug-container {
  height: 100vh !important;
  width: 100vw !important;
  display: flex !important;
  flex-direction: column !important;
  background-color: #1e1e1e !important;
  color: #d4d4d4 !important;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 12px;
  position: fixed !important;
  top: 0 !important;
  left: 0 !important;
  right: 0 !important;
  bottom: 0 !important;
  margin: 0 !important;
  padding: 0 !important;
  z-index: 999999 !important;
}

.debug-header {
  padding: 8px 16px;
  background: #2d2d2d;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid #3e3e3e;
  flex-shrink: 0;
}

.debug-header .title {
  font-weight: bold;
  color: #569cd6;
}

.clear-btn {
  background: #3e3e3e;
  color: white;
  border: none;
  padding: 4px 8px;
  cursor: pointer;
}

.clear-btn:hover {
  background: #505050;
}

.debug-container .log-list {
  flex: 1 !important;
  overflow-y: auto !important;
  padding: 8px !important;
  background-color: #1e1e1e !important;
  display: block !important;
}

.log-item {
  margin-bottom: 4px;
  display: flex !important;
  gap: 8px;
  line-height: 1.4;
  color: inherit;
}

.log-item.tx { color: #ce9178 !important; }
.log-item.rx { color: #b5cea8 !important; }
.log-item.test { color: #569cd6 !important; }

.log-item .ts { color: #808080 !important; flex-shrink: 0; }
.log-item .dir { font-weight: bold; width: 24px; flex-shrink: 0; }
.log-item .content { margin: 0; white-space: pre-wrap; word-break: break-all; color: inherit; }

.header-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.debug-btn {
  background: #0078d4;
  color: white;
  border: none;
  padding: 6px 12px;
  cursor: pointer;
  font-size: 12px;
}

.debug-btn:hover {
  background: #106ebe;
}

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

.panel {
  border: 2px solid #59a9ff;
  background: #f4faff;
  padding: 12px;
  margin-bottom: 12px;
}

.panel h2 {
  margin: 0 0 10px;
  font-size: 18px;
  color: #184470;
}

.row {
  display: flex;
  gap: 8px;
  margin-bottom: 10px;
  align-items: center;
  flex-wrap: wrap;
}

.status {
  margin: 8px 0;
}

.info-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(140px, 1fr));
  gap: 8px;
}

.tracker-column {
  display: grid;
  grid-template-columns: 1fr;
  gap: 6px;
}

.tracker-cell {
  border: 2px solid #77b8ff;
  background: #eef7ff;
  padding: 8px;
  display: flex;
  justify-content: space-between;
}

.tracker-cell.inserted {
  border-color: #2c8d5f;
  background: #dff6ea;
}

.button-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 8px;
  margin-bottom: 10px;
}

.log-list {
  border: 2px solid #77b8ff;
  background: #eef7ff;
  min-height: 80px;
  max-height: 220px;
  overflow-y: auto;
}

.log-line {
  padding: 6px 8px;
  border-bottom: 1px solid #c6e2ff;
  font-family: Consolas, "Courier New", monospace;
  font-size: 12px;
}

select,
button {
  border: 2px solid #59a9ff;
  border-radius: 0;
  background: #ffffff;
  color: #12304f;
  padding: 6px 10px;
  font-size: 14px;
}

button {
  cursor: pointer;
}

button:hover:enabled {
  background: #d7ebff;
}

button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
</style>
