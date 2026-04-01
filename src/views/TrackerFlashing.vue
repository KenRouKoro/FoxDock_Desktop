<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import BaseButton from "../components/ui/BaseButton.vue";
import BasePanel from "../components/ui/BasePanel.vue";
import BaseSelect from "../components/ui/BaseSelect.vue";

type TrackerStatus = {
  id: number;
  inserted: boolean;
  usbPath?: string;
};

type FirmwarePhase =
  | "idle"
  | "ready"
  | "entering_bl"
  | "waiting_bootloader"
  | "copying"
  | "verifying"
  | "success"
  | "error";

type FirmwareMode = "manual" | "auto_slot" | "batch_all";

type FirmwareRunState =
  | "idle"
  | "waiting"
  | "queued"
  | "running"
  | "success"
  | "warning"
  | "skipped"
  | "error";

type FirmwareSlotStatus = {
  id: number;
  inserted: boolean;
  usbPath: string;
  state: FirmwareRunState;
  message: string;
};

const props = defineProps<{
  connectedPortName: string;
  trackers: TrackerStatus[];
  loading: boolean;
  busy: boolean;
  mode: FirmwareMode;
  autoUpdateEnabled: boolean;
  selectedTrackerId: number;
  selectedTrackerInserted: boolean;
  selectedTrackerUsbPath: string;
  fileName: string;
  fileSize: number;
  activeTrackerId: number;
  phase: FirmwarePhase;
  progress: number;
  statusMessage: string;
  slotStatuses: FirmwareSlotStatus[];
}>();

const emit = defineEmits<{
  (e: "setMode", mode: FirmwareMode): void;
  (e: "setTrackerId", id: number): void;
  (e: "selectFile", file: File | null): void;
  (e: "startFlash"): void;
  (e: "toggleAutoUpdate"): void;
  (e: "startBatchFlash"): void;
  (e: "refreshStatus"): void;
}>();

const { t } = useI18n();

const flashableTrackers = computed(() => props.trackers.filter((tracker) => tracker.id <= 5));
const canStartManual = computed(() =>
  Boolean(
    props.connectedPortName &&
      props.fileName &&
      props.selectedTrackerInserted &&
      !props.loading &&
      !props.busy,
  ),
);
const canToggleAuto = computed(
  () =>
    Boolean(
      !props.loading &&
        !props.busy &&
        props.connectedPortName &&
        props.fileName,
    ) || props.autoUpdateEnabled,
);
const canStartBatch = computed(() =>
  Boolean(
    props.connectedPortName && props.fileName && !props.loading && !props.busy,
  ),
);
const phaseLabel = computed(() => t(`flashing.phase_${props.phase}`));
const modeLabel = computed(() => t(`flashing.mode_${props.mode}`));
const activeSlotLabel = computed(() =>
  props.activeTrackerId > 0 ? t("tracker_status.slot", { id: props.activeTrackerId }) : "-",
);

/** 主操作区不可用时的简短原因（不替代 statusMessage） */
const primaryBlockHint = computed(() => {
  if (props.loading) return t("flashing.hint_loading");
  if (!props.connectedPortName) return t("flashing.require_connection");
  if (!props.fileName) return t("flashing.require_file");
  if (props.mode === "manual" && !props.selectedTrackerInserted) {
    return t("flashing.require_inserted_tracker", { id: props.selectedTrackerId });
  }
  return "";
});

const showPrimaryBlockHint = computed(() => {
  if (!primaryBlockHint.value) return false;
  if (props.mode === "manual") return !canStartManual.value;
  if (props.mode === "auto_slot") return !canToggleAuto.value;
  return !canStartBatch.value;
});

function handleFileChange(event: Event): void {
  const target = event.target as HTMLInputElement;
  const file = target.files?.[0] ?? null;
  emit("selectFile", file);
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}
</script>

<template>
  <div class="flashing-page">
    <BasePanel :title="t('flashing.title')">
      <div class="flashing-layout">
        <!-- 1. 概览条 -->
        <div class="summary-bar" :class="{ 'summary-bar--ok': connectedPortName }">
          <div class="summary-main">
            <span class="summary-title">{{ t("flashing.summary_title") }}</span>
            <span v-if="connectedPortName" class="summary-dock mono">
              {{ connectedPortName }}
            </span>
            <span v-else class="summary-dock summary-dock--bad">
              {{ t("flashing.summary_disconnected") }}
            </span>
          </div>
          <div class="summary-sub">
            <span class="summary-mode">{{ modeLabel }}</span>
            <span v-if="fileName" class="summary-file mono" :title="fileName">{{ fileName }}</span>
            <span v-else class="summary-file summary-file--empty">{{ t("flashing.file_empty_short") }}</span>
          </div>
        </div>

        <!-- 2. 任务配置 -->
        <section class="block block--config">
          <h3 class="block-heading">{{ t("flashing.config_title") }}</h3>
          <p class="limit-strip">{{ t("flashing.slot_limit_short") }}</p>

          <div class="field-grid">
            <div class="field">
              <label class="field-label" for="firmwareMode">{{ t("flashing.mode_label") }}</label>
              <BaseSelect
                id="firmwareMode"
                :model-value="mode"
                :disabled="loading || busy"
                class="field-control"
                @update:model-value="emit('setMode', $event as FirmwareMode)"
              >
                <option value="manual">{{ t("flashing.mode_manual") }}</option>
                <option value="auto_slot">{{ t("flashing.mode_auto_slot") }}</option>
                <option value="batch_all">{{ t("flashing.mode_batch_all") }}</option>
              </BaseSelect>
            </div>
            <div class="field">
              <label class="field-label" for="firmwareTracker">{{ t("flashing.slot_label") }}</label>
              <div class="field-row-inner">
                <BaseSelect
                  id="firmwareTracker"
                  :model-value="selectedTrackerId"
                  :disabled="loading || busy || mode === 'batch_all'"
                  class="field-control field-control--grow"
                  @update:model-value="emit('setTrackerId', Number($event))"
                >
                  <option
                    v-for="tracker in flashableTrackers"
                    :key="tracker.id"
                    :value="tracker.id"
                  >
                    {{ t("tracker_status.slot", { id: tracker.id }) }}
                  </option>
                </BaseSelect>
                <BaseButton
                  variant="outline"
                  :disabled="loading || busy || !connectedPortName"
                  @click="emit('refreshStatus')"
                >
                  {{ t("tracker_control.refresh_status") }}
                </BaseButton>
              </div>
            </div>
          </div>

          <p v-if="mode === 'batch_all'" class="mode-hint">{{ t("flashing.batch_target_hint") }}</p>
          <p v-else class="mode-hint mode-hint--muted">{{ t(`flashing.mode_hint_${mode}`) }}</p>

          <div class="target-strip">
            <div class="target-strip-row">
              <span class="target-strip-label">{{ t("flashing.slot_state") }}</span>
              <strong
                class="target-strip-value"
                :class="selectedTrackerInserted ? 'text-ok' : 'text-bad'"
              >
                {{
                  selectedTrackerInserted
                    ? t("tracker_status.inserted")
                    : t("tracker_status.not_inserted")
                }}
              </strong>
            </div>
            <div v-if="selectedTrackerUsbPath" class="target-strip-row">
              <span class="target-strip-label">{{ t("flashing.usb_path_label") }}</span>
              <span class="mono target-path">{{ selectedTrackerUsbPath }}</span>
            </div>
          </div>

          <div class="file-section">
            <label class="file-label" for="firmwareFile">{{ t("flashing.file_picker_label") }}</label>
            <input
              id="firmwareFile"
              class="file-input"
              type="file"
              accept=".uf2"
              :disabled="loading || busy"
              @change="handleFileChange"
            />
            <div class="file-card">
              <template v-if="fileName">
                <div class="file-meta">
                  <span>{{ t("flashing.file_name_label") }}</span>
                  <strong class="mono file-name">{{ fileName }}</strong>
                </div>
                <div class="file-meta">
                  <span>{{ t("flashing.file_size_label") }}</span>
                  <strong class="mono">{{ formatFileSize(fileSize) }}</strong>
                </div>
              </template>
              <p v-else class="file-empty">{{ t("flashing.file_empty") }}</p>
            </div>
          </div>
        </section>

        <!-- 3. 执行状态与主操作 -->
        <section class="block block--execution">
          <h3 class="block-heading">{{ t("flashing.execution_title") }}</h3>
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
            <div class="progress-bar-container">
              <div class="progress-bar" :style="{ width: `${Math.min(progress, 100)}%` }" />
            </div>
            <p class="status-message">{{ statusMessage }}</p>
            <p v-if="showPrimaryBlockHint" class="block-hint">{{ primaryBlockHint }}</p>
            <div class="action-row">
              <BaseButton v-if="mode === 'manual'" :disabled="!canStartManual" @click="emit('startFlash')">
                {{ busy ? t("flashing.busy") : t("flashing.start") }}
              </BaseButton>
              <BaseButton
                v-else-if="mode === 'auto_slot'"
                :disabled="!canToggleAuto"
                @click="emit('toggleAutoUpdate')"
              >
                {{ autoUpdateEnabled ? t("flashing.disable_auto") : t("flashing.enable_auto") }}
              </BaseButton>
              <BaseButton v-else :disabled="!canStartBatch" @click="emit('startBatchFlash')">
                {{ busy ? t("flashing.busy") : t("flashing.start_batch") }}
              </BaseButton>
            </div>
          </div>
        </section>

        <!-- 4. 槽位队列 -->
        <section class="block block--queue">
          <h3 class="block-heading">{{ t("flashing.queue_title") }}</h3>
          <div class="slot-list">
            <div
              v-for="slot in slotStatuses"
              :key="slot.id"
              class="slot-card"
              :class="[
                `slot-card--${slot.state}`,
                { 'slot-card--active': activeTrackerId === slot.id },
              ]"
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
        </section>
      </div>
    </BasePanel>
  </div>
</template>

<style scoped>
.mono {
  font-family: var(--font-family-mono);
  font-size: 12px;
  word-break: break-all;
}

.flashing-page {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
}

.flashing-layout {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-lg);
}

/* --- 概览条 --- */
.summary-bar {
  border: var(--border-width) solid var(--color-secondary);
  background: var(--color-bg-header);
  padding: var(--spacing-sm) var(--spacing-md);
  box-shadow: var(--box-shadow);
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}

.summary-bar--ok {
  border-color: var(--color-success-border);
}

.summary-main {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: var(--spacing-sm);
}

.summary-title {
  font-size: 12px;
  font-weight: 700;
  color: var(--color-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.02em;
}

.summary-dock {
  font-size: 12px;
  color: var(--color-text-main);
}

.summary-dock--bad {
  color: var(--color-error);
  font-weight: 700;
}

.summary-sub {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--spacing-sm);
  font-size: 12px;
  color: var(--color-text-light);
}

.summary-mode {
  padding: 2px 6px;
  border: 1px solid var(--color-primary);
  background: var(--color-bg-white);
  color: var(--color-primary);
  font-weight: 600;
}

.summary-file {
  flex: 1;
  min-width: 0;
  color: var(--color-text-secondary);
}

.summary-file--empty {
  color: var(--color-text-light);
  font-style: italic;
}

/* --- 通用区块 --- */
.block {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
}

.block-heading {
  margin: 0;
  font-size: 13px;
  font-weight: 700;
  color: var(--color-text-secondary);
  padding-bottom: var(--spacing-xs);
  border-bottom: var(--border-width) solid var(--color-secondary-hover);
}

.limit-strip {
  margin: 0;
  padding: var(--spacing-xs) var(--spacing-sm);
  border: 1px dashed var(--color-secondary);
  background: var(--color-bg-white);
  font-size: 12px;
  line-height: 1.45;
  color: var(--color-text-light);
}

.field-grid {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
}

.field {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}

.field-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-secondary);
}

.field-row-inner {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-sm);
  align-items: center;
}

.field-control {
  min-width: 140px;
}

.field-control--grow {
  flex: 1;
  min-width: 0;
}

.mode-hint {
  margin: 0;
  font-size: 12px;
  line-height: 1.45;
  color: var(--color-text-main);
}

.mode-hint--muted {
  color: var(--color-text-light);
}

.target-strip {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
  padding: var(--spacing-sm);
  border: var(--border-width) solid var(--color-secondary-hover);
  background: var(--color-bg-white);
}

.target-strip-row {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--spacing-sm);
  font-size: 12px;
}

.target-strip-label {
  color: var(--color-text-light);
}

.target-strip-value {
  font-size: 13px;
}

.target-path {
  text-align: right;
  max-width: 100%;
}

.text-ok {
  color: var(--color-success);
}

.text-bad {
  color: var(--color-error);
}

/* --- 固件文件 --- */
.file-section {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}

.file-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-secondary);
}

.file-input {
  border: var(--border-width) solid var(--color-border-control);
  border-radius: var(--border-radius);
  background: var(--color-bg-white);
  color: var(--color-text-main);
  font-size: 14px;
  line-height: 1.25;
  width: 100%;
  max-width: 100%;
  min-height: 32px;
  padding: 0 var(--spacing-sm) 0 0;
  cursor: pointer;
  transition: border-color 0.2s, box-shadow 0.2s;
  outline: none;
}

.file-input:focus-visible:not(:disabled) {
  border-color: var(--color-primary);
  box-shadow: inset 0 0 0 1px var(--color-primary);
}

.file-input:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.file-input::file-selector-button {
  border: none;
  border-right: var(--border-width) solid var(--color-border-control);
  background: var(--color-bg-white);
  color: var(--color-primary);
  padding: 6px 12px;
  margin: 0 var(--spacing-sm) 0 0;
  font-size: 14px;
  line-height: 1.25;
  font-family: inherit;
  cursor: pointer;
  transition: background-color 0.2s, color 0.2s;
}

.file-input:hover:not(:disabled)::file-selector-button {
  background: var(--color-bg-control-hover);
}

.file-input:disabled::file-selector-button {
  cursor: not-allowed;
}

.file-card {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
  padding: var(--spacing-md);
  border: var(--border-width) dashed var(--color-secondary);
  background: var(--color-bg-header);
}

.file-meta {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--spacing-sm);
  flex-wrap: wrap;
  font-size: 12px;
}

.file-name {
  text-align: right;
  max-width: 65%;
}

.file-empty {
  margin: 0;
  color: var(--color-text-light);
  font-size: 12px;
}

/* --- 执行区 --- */
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

.progress-bar-container {
  width: 100%;
  height: 12px;
  background: var(--color-bg-page);
  border: var(--border-width) solid var(--color-primary);
}

.progress-bar {
  height: 100%;
  background: var(--color-primary);
  transition: width 0.2s ease;
}

.status-message {
  margin: 0;
  font-size: 12px;
  line-height: 1.5;
  color: var(--color-text-secondary);
}

.block-hint {
  margin: 0;
  font-size: 12px;
  color: var(--color-error);
  padding: var(--spacing-xs) var(--spacing-sm);
  border: 1px solid var(--color-error);
  background: var(--color-error-bg);
}

.action-row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-sm);
  padding-top: var(--spacing-xs);
}

/* --- 槽位队列 --- */
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
  border: 1px solid var(--color-secondary);
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

/* 槽位卡片行背景弱提示 */
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
