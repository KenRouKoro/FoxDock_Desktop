<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { BleOtaPhase } from "../../types/firmware";
import BaseProgressBar from "../ui/BaseProgressBar.vue";

const props = defineProps<{
  phase: BleOtaPhase | string;
  progress: number;
  message: string;
  bytesTransferred?: number;
  totalBytes?: number;
}>();

const { t } = useI18n();

const phaseLabel = computed(() => {
  const p = props.phase as string;
  const key = `ble_ota.phase_${p}`;
  const translated = t(key);
  return translated === key ? p : translated;
});
</script>

<template>
  <div class="ble-ota-progress" role="status">
    <div class="phase-row">
      <span class="label">{{ t("ble_ota.current_phase") }}</span>
      <strong>{{ phaseLabel }}</strong>
    </div>
    <BaseProgressBar :progress="progress" />
    <p class="msg">{{ message }}</p>
    <p v-if="bytesTransferred != null && totalBytes != null" class="bytes mono">
      {{ t("ble_ota.bytes_progress", { current: bytesTransferred, total: totalBytes }) }}
    </p>
  </div>
</template>

<style scoped>
.ble-ota-progress {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
}
.phase-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 12px;
}
.label {
  color: var(--color-text-light);
}
.msg {
  margin: 0;
  font-size: 12px;
  line-height: 1.4;
  color: var(--color-text-main);
}
.bytes {
  margin: 0;
  font-size: 11px;
  color: var(--color-text-secondary);
}
</style>
