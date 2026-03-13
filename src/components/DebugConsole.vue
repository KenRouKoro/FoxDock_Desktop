<script setup lang="ts">
import { ref, onMounted, nextTick } from "vue";
import { listen } from "@tauri-apps/api/event";

interface DebugLog {
  direction: string;
  content: string;
  timestamp: string;
}

const debugLogs = ref<DebugLog[]>([]);
const logContainer = ref<HTMLElement | null>(null);

onMounted(async () => {
  await listen<DebugLog>("serial-debug-log", (event) => {
    const newLog = event.payload;
    const lastLog = debugLogs.value[debugLogs.value.length - 1];

    if (lastLog && lastLog.direction === newLog.direction) {
      lastLog.content += newLog.content;
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
  });
});

const clearLogs = () => {
  debugLogs.value = [];
};
</script>

<template>
  <main class="debug-container">
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
</template>

<style scoped>
.debug-container {
  height: 100vh !important;
  width: 100vw !important;
  display: flex !important;
  flex-direction: column !important;
  background-color: #1e1e1e !important;
  color: #d4d4d4 !important;
  font-family: var(--font-family-mono);
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

.log-list {
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
</style>
