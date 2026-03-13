<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { ref, watch } from "vue";
import BaseButton from "./ui/BaseButton.vue";
import BasePanel from "./ui/BasePanel.vue";
import BaseSelect from "./ui/BaseSelect.vue";

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
  <BasePanel :title="t('connection.title')">
    <div class="row">
      <BaseSelect v-model="selectedPortName" :disabled="loading">
        <option value="">{{ t('connection.select_port') }}</option>
        <option v-for="dock in docks" :key="dock.portName" :value="dock.portName">
          {{ dock.displayName }}
        </option>
      </BaseSelect>
      <BaseButton :disabled="loading" variant="outline" @click="emit('refresh')">{{ t('common.refresh') }}</BaseButton>
      <BaseButton :disabled="loading || !selectedPortName || !!connectedPortName" @click="emit('connect', selectedPortName)">{{ t('common.connect') }}</BaseButton>
      <BaseButton :disabled="loading || !connectedPortName" @click="emit('disconnect')">{{ t('common.disconnect') }}</BaseButton>
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
  </BasePanel>
</template>

<style scoped>
.row {
  display: flex;
  gap: var(--spacing-sm);
  margin-bottom: var(--spacing-sm);
  align-items: center;
  flex-wrap: wrap;
}

.status {
  margin: var(--spacing-sm) 0;
}

.connected {
  color: var(--color-success);
  font-weight: bold;
}

.disconnected {
  color: var(--color-error);
}

.info-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(140px, 1fr));
  gap: var(--spacing-sm);
}
</style>
