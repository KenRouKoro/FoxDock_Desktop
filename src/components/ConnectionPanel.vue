<script setup lang="ts">
import { useI18n } from "vue-i18n";
const { t } = useI18n();

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

const props = defineProps<{
  docks: DockPort[];
  connectedPortName: string;
  dockInfo: DockInfo | null;
  loading: boolean;
}>();

const emit = defineEmits<{
  (e: 'refresh'): void;
  (e: 'connect', portName: string): void;
  (e: 'disconnect'): void;
}>();

import { ref, watch } from "vue";
const selectedPortName = ref("");

watch(
  () => props.connectedPortName,
  (value) => {
    if (value) {
      selectedPortName.value = value;
    }
  },
  { immediate: true }
);
</script>

<template>
  <section class="panel">
    <h2>{{ t('connection.title') }}</h2>
    <div class="row">
      <select v-model="selectedPortName" :disabled="loading">
        <option value="">{{ t('connection.select_port') }}</option>
        <option v-for="dock in docks" :key="dock.portName" :value="dock.portName">
          {{ dock.displayName }}
        </option>
      </select>
      <button :disabled="loading" @click="emit('refresh')">{{ t('common.refresh') }}</button>
      <button :disabled="loading || !selectedPortName || !!connectedPortName" @click="emit('connect', selectedPortName)">{{ t('common.connect') }}</button>
      <button :disabled="loading || !connectedPortName" @click="emit('disconnect')">{{ t('common.disconnect') }}</button>
    </div>
    <p class="status">
      {{ t('connection.current_status') }}
      <span v-if="connectedPortName" class="connected">{{ t('connection.connected') }} ({{ connectedPortName }})</span>
      <span v-else class="disconnected">{{ t('connection.disconnected') }}</span>
    </p>
    <div class="info-grid">
      <div>{{ t('connection.project') }} {{ dockInfo?.project ?? "-" }}</div>
      <div>{{ t('connection.version') }} {{ dockInfo?.version ?? "-" }}</div>
      <div>{{ t('connection.mcu') }} {{ dockInfo?.mcu ?? "-" }}</div>
    </div>
  </section>
</template>

<style scoped>
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

.connected {
  color: #107c10;
  font-weight: bold;
}

.disconnected {
  color: #d83b01;
}

.info-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(140px, 1fr));
  gap: 8px;
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
