<script setup lang="ts">
import { useI18n } from "vue-i18n";
import BaseButton from "../ui/BaseButton.vue";
import BaseProgressBar from "../ui/BaseProgressBar.vue";
import type { FirmwareMode } from "../../types/firmware";

defineProps<{
  mode: FirmwareMode;
  busy: boolean;
  disabled: boolean;
  activeSlotLabel: string;
  phaseLabel: string;
  progress: number;
  statusMessage: string;
  canStartManual: boolean;
  canToggleAuto: boolean;
  canStartBatch: boolean;
  autoUpdateEnabled: boolean;
}>();

const emit = defineEmits<{
  (e: "startFlash"): void;
  (e: "toggleAutoUpdate"): void;
  (e: "startBatchFlash"): void;
}>();

const { t } = useI18n();
</script>

<template>
  <div class="execution-card" :class="{ 'execution-card--busy': busy }">
    <div class="exec-metrics">
      <div class="exec-metric">
        <span class="exec-metric-label">{{ t("flashing.active_slot_label") }}</span>
        <strong class="exec-metric-value">{{ activeSlotLabel }}</strong>
      </div>
      <div class="exec-metric">
        <span class="exec-metric-label">{{ t("flashing.phase_label") }}</span>
        <strong class="exec-metric-value">{{ phaseLabel }}</strong>
      </div>
      <div class="exec-metric exec-metric--progress">
        <span class="exec-metric-label">{{ t("flashing.progress_label") }}</span>
        <strong class="exec-metric-value mono">{{ progress }}%</strong>
      </div>
    </div>
    <BaseProgressBar :progress="progress" />
    <p class="status-message">{{ statusMessage }}</p>
    <div class="action-row">
      <BaseButton
        v-if="mode === 'manual'"
        :disabled="disabled || !canStartManual"
        @click="emit('startFlash')"
      >
        {{ busy ? t("flashing.busy") : t("flashing.start") }}
      </BaseButton>
      <BaseButton
        v-else-if="mode === 'auto_slot'"
        :disabled="disabled || !canToggleAuto"
        @click="emit('toggleAutoUpdate')"
      >
        {{ autoUpdateEnabled ? t("flashing.disable_auto") : t("flashing.enable_auto") }}
      </BaseButton>
      <BaseButton v-else :disabled="disabled || !canStartBatch" @click="emit('startBatchFlash')">
        {{ busy ? t("flashing.busy") : t("flashing.start_batch") }}
      </BaseButton>
    </div>
  </div>
</template>

<style scoped>
.execution-card {
  border: var(--border-width) solid var(--border-color);
  background: var(--color-bg-white);
  padding: var(--spacing-md);
  box-shadow: var(--box-shadow);
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
}

.execution-card--busy {
  border-color: var(--color-primary);
  box-shadow: var(--box-shadow-heavy);
}

.exec-metrics {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--spacing-sm);
}

.exec-metric {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.exec-metric--progress {
  grid-column: 1 / -1;
}

.exec-metric-label {
  font-size: 11px;
  color: var(--color-text-light);
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.exec-metric-value {
  font-size: 14px;
  color: var(--color-text-main);
}

.status-message {
  margin: 0;
  font-size: 12px;
  line-height: 1.5;
  color: var(--color-text-secondary);
}

.action-row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-sm);
  padding-top: var(--spacing-xs);
}
</style>
