<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { FirmwareSlotStatus } from "../../types/firmware";

defineProps<{
  slotStatuses: FirmwareSlotStatus[];
  activeTrackerId: number;
}>();

const { t } = useI18n();
</script>

<template>
  <div class="slot-list">
    <div
      v-for="slot in slotStatuses"
      :key="slot.id"
      class="slot-card"
      :class="[`slot-card--${slot.state}`, { 'slot-card--active': activeTrackerId === slot.id }]"
    >
      <div class="slot-card-head">
        <strong class="slot-name">{{ t("tracker_status.slot", { id: slot.id }) }}</strong>
        <span class="slot-pill" :class="`slot-pill--${slot.state}`">
          {{ t(`flashing.slot_state_${slot.state}`) }}
        </span>
      </div>
      <div class="slot-card-meta">
        <span>{{ slot.inserted ? t("tracker_status.inserted") : t("tracker_status.not_inserted") }}</span>
        <span v-if="slot.usbPath" class="mono slot-usb">{{ slot.usbPath }}</span>
      </div>
      <p class="slot-message">{{ slot.message }}</p>
    </div>
  </div>
</template>

<style scoped>
.mono {
  font-family: var(--font-family-mono);
  font-size: 11px;
  word-break: break-all;
}

.slot-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}

.slot-card {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
  padding: var(--spacing-sm);
  border: var(--border-width) solid var(--border-color);
  background: var(--color-bg-white);
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.slot-card--active {
  border-color: var(--color-primary);
  box-shadow: var(--box-shadow);
}

.slot-card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-sm);
  flex-wrap: wrap;
}

.slot-name {
  font-size: 13px;
  color: var(--color-text-secondary);
}

.slot-pill {
  font-size: 11px;
  font-weight: 700;
  padding: 2px 8px;
  border: var(--border-width-subtle) solid var(--color-secondary);
  background: var(--color-bg-header);
  color: var(--color-text-light);
}

.slot-pill--running {
  border-color: var(--color-primary);
  background: var(--color-secondary-hover);
  color: var(--color-primary);
}

.slot-pill--success,
.slot-pill--warning {
  border-color: var(--color-success-border);
  background: var(--color-success-bg);
  color: var(--color-success);
}

.slot-pill--error {
  border-color: var(--color-error);
  background: var(--color-error-bg);
  color: var(--color-error);
}

.slot-pill--queued {
  border-color: var(--color-secondary);
  color: var(--color-primary);
}

.slot-card-meta {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: var(--spacing-sm);
  font-size: 11px;
  color: var(--color-text-light);
}

.slot-usb {
  text-align: right;
  max-width: 100%;
  color: var(--color-secondary);
}

.slot-message {
  margin: 0;
  font-size: 11px;
  line-height: 1.4;
  color: var(--color-text-main);
}

.slot-card--running {
  background: var(--color-secondary-hover);
}

.slot-card--success,
.slot-card--warning {
  background: var(--color-success-bg);
}

.slot-card--error {
  background: var(--color-error-bg);
}

.slot-card--skipped,
.slot-card--idle,
.slot-card--waiting,
.slot-card--queued {
  background: var(--color-bg-white);
}
</style>
