<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { BleDfuDevice } from "../../types/firmware";
defineProps<{
  devices: BleDfuDevice[];
  selectedId: string;
  disabled: boolean;
}>();

const emit = defineEmits<{
  (e: "select", id: string): void;
}>();

const { t } = useI18n();
</script>

<template>
  <div class="ble-device-list" role="list">
    <p v-if="devices.length === 0" class="empty">{{ t("ble_ota.no_devices") }}</p>
    <ul v-else class="list">
      <li v-for="d in devices" :key="d.peripheralId" class="row">
        <label class="radio-label">
          <input
            type="radio"
            name="ble-dfu-device"
            :value="d.peripheralId"
            :checked="selectedId === d.peripheralId"
            :disabled="disabled"
            @change="emit('select', d.peripheralId)"
          />
          <span class="name">{{ d.name }}</span>
          <span class="addr mono">{{ d.address }}</span>
          <span v-if="d.rssi != null" class="rssi">{{ d.rssi }} dBm</span>
        </label>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.empty {
  margin: 0;
  font-size: 12px;
  color: var(--color-text-light);
}
.list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}
.row {
  margin: 0;
}
.radio-label {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--spacing-sm);
  font-size: 12px;
  cursor: pointer;
}
.name {
  font-weight: 600;
  color: var(--color-text-main);
}
.addr {
  color: var(--color-text-secondary);
  flex: 1;
  min-width: 0;
}
.rssi {
  color: var(--color-text-light);
}
.addr.mono {
  word-break: break-all;
}
</style>
