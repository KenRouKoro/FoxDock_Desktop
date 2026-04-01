<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import ConnectionPanel from "../components/ConnectionPanel.vue";
import FirmwareExecutionCard from "../components/firmware/FirmwareExecutionCard.vue";
import FirmwareSlotQueue from "../components/firmware/FirmwareSlotQueue.vue";
import BaseButton from "../components/ui/BaseButton.vue";
import BasePanel from "../components/ui/BasePanel.vue";
import BaseSelect from "../components/ui/BaseSelect.vue";
import logoUrl from "../assets/FoxApplication.png";
import type { DockInfo, DockPort, TrackerStatus } from "../types/dock";
import type { FirmwareMode, FirmwarePhase, FirmwareSlotStatus } from "../types/firmware";

const props = defineProps<{
  connectedPortName: string;
  docks: DockPort[];
  dockInfo: DockInfo | null;
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
  (e: "refreshDocks"): void;
  (e: "connectDock", portName: string): void;
  (e: "disconnectDock"): void;
}>();

const { t } = useI18n();

type FlashingTab = "config" | "status";

const activeTab = ref<FlashingTab>("config");
const isConnectionExpanded = ref(true);

const flashableTrackers = computed(() => props.trackers.filter((tracker) => tracker.id <= 5));

const insertedFlashableCount = computed(
  () => flashableTrackers.value.filter((t) => t.inserted).length,
);

const controlsLocked = computed(() => props.loading || props.busy);

watch(
  () => props.connectedPortName,
  (name, oldName) => {
    if (!name) {
      isConnectionExpanded.value = true;
    } else if (oldName === undefined || oldName === "") {
      isConnectionExpanded.value = false;
    }
  },
  { immediate: true },
);

function toggleConnectionPanel() {
  isConnectionExpanded.value = !isConnectionExpanded.value;
}

function setTab(tab: FlashingTab) {
  activeTab.value = tab;
}

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
  Boolean(props.connectedPortName && props.fileName && !props.loading && !props.busy),
);

const phaseLabel = computed(() => t(`flashing.phase_${props.phase}`));
const modeLabel = computed(() => t(`flashing.mode_${props.mode}`));
const activeSlotLabel = computed(() =>
  props.activeTrackerId > 0 ? t("tracker_status.slot", { id: props.activeTrackerId }) : "-",
);

/** 主操作不可用时的简短原因（摘要条与执行区一致） */
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
  <div class="flashing-view">
    <header class="header">
      <div class="header-bar">
        <div class="header-title-area">
          <img :src="logoUrl" class="logo" alt="FoxDock Logo" />
          <div class="header-text">
            <h1>{{ t("flashing.title") }}</h1>
            <p>{{ t("flashing.description") }}</p>
          </div>
        </div>
      </div>
    </header>

    <section
      class="connection-bar"
      :class="{
        'connection-bar--ok': !!connectedPortName,
        'connection-bar--expanded': isConnectionExpanded,
      }"
      :aria-label="t('flashing.connection_bar_label')"
    >
      <div class="connection-bar__header-row">
        <div class="connection-bar__summary" role="status">
          <span v-if="connectedPortName" class="summary-line mono">
            {{ t("home.summary_port", { port: connectedPortName }) }}
          </span>
          <span v-else class="summary-line summary-line--emphasis">
            {{ t("home.summary_disconnected") }}
          </span>
          <span class="summary-line">
            {{ t("flashing.summary_inserted_1_5", { count: insertedFlashableCount }) }}
          </span>
        </div>
        <div class="connection-bar__actions">
          <BaseButton
            variant="outline"
            :disabled="loading || !connectedPortName"
            @click="emit('refreshStatus')"
          >
            {{ t("tracker_control.refresh_status") }}
          </BaseButton>
          <button
            type="button"
            class="connection-bar__toggle"
            :aria-expanded="isConnectionExpanded"
            aria-controls="flashing-connection-panel"
            @click="toggleConnectionPanel"
          >
            {{
              isConnectionExpanded
                ? t("home.connection_bar_collapse")
                : t("home.connection_bar_expand")
            }}
          </button>
        </div>
      </div>

      <p v-if="showPrimaryBlockHint" class="connection-bar__hint" role="status">
        {{ primaryBlockHint }}
      </p>

      <div class="connection-bar__meta">
        <span class="summary-mode">{{ modeLabel }}</span>
        <span v-if="fileName" class="summary-file mono" :title="fileName">{{ fileName }}</span>
        <span v-else class="summary-file summary-file--empty">{{ t("flashing.file_empty_short") }}</span>
      </div>

      <div
        class="connection-bar__panel-outer"
        :class="{ 'connection-bar__panel-outer--open': isConnectionExpanded }"
        :aria-hidden="!isConnectionExpanded"
      >
        <div class="connection-bar__panel-inner">
          <div id="flashing-connection-panel" class="connection-bar__panel">
            <ConnectionPanel
              embedded
              :docks="docks"
              :connected-port-name="connectedPortName"
              :dock-info="dockInfo"
              :loading="loading"
              @refresh="emit('refreshDocks')"
              @connect="(port) => emit('connectDock', port)"
              @disconnect="emit('disconnectDock')"
            />
          </div>
        </div>
      </div>
    </section>

    <nav class="flashing-tabs" role="tablist" :aria-label="t('flashing.tabs_aria_label')">
      <button
        type="button"
        role="tab"
        class="flashing-tab"
        :class="{ 'flashing-tab--active': activeTab === 'config' }"
        :aria-selected="activeTab === 'config'"
        @click="setTab('config')"
      >
        {{ t("flashing.tab_config") }}
      </button>
      <button
        type="button"
        role="tab"
        class="flashing-tab"
        :class="{ 'flashing-tab--active': activeTab === 'status' }"
        :aria-selected="activeTab === 'status'"
        @click="setTab('status')"
      >
        {{ t("flashing.tab_status") }}
      </button>
    </nav>

    <div v-show="activeTab === 'config'" class="flashing-panels" role="tabpanel">
      <BasePanel :title="t('flashing.config_title')">
        <p class="limit-strip">{{ t("flashing.slot_limit_short") }}</p>

        <div class="field-grid">
          <div class="field">
            <label class="field-label" for="firmwareMode">{{ t("flashing.mode_label") }}</label>
            <BaseSelect
              id="firmwareMode"
              :model-value="mode"
              :disabled="controlsLocked"
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
            <BaseSelect
              id="firmwareTracker"
              :model-value="selectedTrackerId"
              :disabled="controlsLocked || mode === 'batch_all'"
              class="field-control"
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
            :disabled="controlsLocked"
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
      </BasePanel>
    </div>

    <div v-show="activeTab === 'status'" class="flashing-panels" role="tabpanel">
      <BasePanel :title="t('flashing.execution_title')">
        <FirmwareExecutionCard
          :mode="mode"
          :busy="busy"
          :disabled="loading"
          :active-slot-label="activeSlotLabel"
          :phase-label="phaseLabel"
          :progress="progress"
          :status-message="statusMessage"
          :can-start-manual="canStartManual"
          :can-toggle-auto="canToggleAuto"
          :can-start-batch="canStartBatch"
          :auto-update-enabled="autoUpdateEnabled"
          @start-flash="emit('startFlash')"
          @toggle-auto-update="emit('toggleAutoUpdate')"
          @start-batch-flash="emit('startBatchFlash')"
        />
      </BasePanel>
      <BasePanel :title="t('flashing.queue_title')">
        <FirmwareSlotQueue :slot-statuses="slotStatuses" :active-tracker-id="activeTrackerId" />
      </BasePanel>
    </div>
  </div>
</template>

<style scoped>
.flashing-view {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
}

.header {
  border: var(--border-width) solid var(--color-secondary);
  background: var(--color-bg-header);
  padding: var(--spacing-xs) var(--spacing-md);
}

.header-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.header-title-area {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
}

.logo {
  height: 28px;
  width: auto;
}

.header-text h1 {
  margin: 0;
  font-size: 15px;
  line-height: 1.2;
}

.header-text p {
  margin: 0;
  font-size: 11px;
  color: var(--color-text-light);
}

.connection-bar {
  display: flex;
  flex-direction: column;
  border: var(--border-width) solid var(--color-secondary);
  background: var(--color-bg-panel);
  font-size: 12px;
  line-height: 1.35;
}

.connection-bar--ok {
  border-color: var(--color-success-border);
}

.connection-bar__header-row {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--spacing-sm);
  padding: var(--spacing-sm) var(--spacing-md);
}

.connection-bar__summary {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
  flex: 1;
  min-width: 0;
}

.connection-bar__actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--spacing-sm);
  flex-shrink: 0;
}

.connection-bar__hint {
  margin: 0 var(--spacing-md) var(--spacing-sm);
  padding: var(--spacing-xs) var(--spacing-sm);
  font-size: 12px;
  color: var(--color-error);
  border: var(--border-width-subtle) solid var(--color-error);
  background: var(--color-error-bg);
}

.connection-bar__meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--spacing-sm);
  padding: 0 var(--spacing-md) var(--spacing-sm);
  font-size: 12px;
  color: var(--color-text-light);
}

.connection-bar__toggle {
  margin: 0;
  padding: var(--spacing-xs) var(--spacing-sm);
  border: var(--border-width) solid var(--color-secondary);
  background: var(--color-bg-header);
  color: var(--color-primary);
  font-size: 12px;
  font-weight: 700;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
}

.connection-bar__toggle:hover {
  background: var(--color-secondary-hover);
}

.connection-bar__panel-outer {
  display: grid;
  grid-template-rows: 0fr;
  transition: grid-template-rows 0.28s cubic-bezier(0.4, 0, 0.2, 1);
}

.connection-bar__panel-outer--open {
  grid-template-rows: 1fr;
}

.connection-bar__panel-inner {
  overflow: hidden;
  min-height: 0;
}

.connection-bar__panel {
  border-top: var(--border-width-subtle) solid var(--color-secondary-hover);
  padding: 0 var(--spacing-md) var(--spacing-sm);
  opacity: 0;
  transition: opacity 0.2s ease;
}

.connection-bar__panel-outer--open .connection-bar__panel {
  opacity: 1;
  transition: opacity 0.22s ease 0.04s;
}

.summary-line {
  color: var(--color-text-main);
}

.summary-line--emphasis {
  color: var(--color-error);
  font-weight: 700;
}

.summary-mode {
  padding: 2px 6px;
  border: var(--border-width-subtle) solid var(--color-primary);
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

.mono {
  font-family: var(--font-family-mono);
  font-size: 12px;
  word-break: break-all;
}

.flashing-tabs {
  display: flex;
  border: var(--border-width) solid var(--color-secondary);
  background: var(--color-bg-header);
}

.flashing-tab {
  flex: 1;
  margin: 0;
  padding: var(--spacing-sm) var(--spacing-xs);
  border: none;
  border-right: var(--border-width-subtle) solid var(--color-secondary-hover);
  background: transparent;
  color: var(--color-text-light);
  font-size: 12px;
  font-weight: 700;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
}

.flashing-tab:last-child {
  border-right: none;
}

.flashing-tab:hover {
  background: var(--color-secondary-hover);
  color: var(--color-primary);
}

.flashing-tab--active {
  background: var(--color-bg-white);
  color: var(--color-primary);
  box-shadow: inset 0 -3px 0 var(--color-primary);
}

.flashing-panels {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
  min-height: 0;
}

.limit-strip {
  margin: 0 0 var(--spacing-sm);
  padding: var(--spacing-xs) var(--spacing-sm);
  border: var(--border-width-subtle) dashed var(--color-secondary);
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

.field-control {
  min-width: 140px;
  width: 100%;
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
  box-shadow: inset 0 0 0 var(--border-width-subtle) var(--color-primary);
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
</style>
