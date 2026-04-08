<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";

defineOptions({ name: "TrackerFlashing" });
import { useI18n } from "vue-i18n";
import ConnectionPanel from "../components/ConnectionPanel.vue";
import BleOtaPanel from "../components/ble-ota/BleOtaPanel.vue";
import FirmwareExecutionCard from "../components/firmware/FirmwareExecutionCard.vue";
import FirmwareSlotQueue from "../components/firmware/FirmwareSlotQueue.vue";
import BaseButton from "../components/ui/BaseButton.vue";
import BasePanel from "../components/ui/BasePanel.vue";
import BaseSelect from "../components/ui/BaseSelect.vue";
import BaseTabs from "../components/ui/BaseTabs.vue";
import PageHeader from "../components/ui/PageHeader.vue";
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
type FirmwareSection = "usb" | "ble_ota";

const FIRMWARE_SECTION_KEY = "foxdock_firmware_section";

const firmwareSection = ref<FirmwareSection>("usb");
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

watch(firmwareSection, (section) => {
  try {
    localStorage.setItem(FIRMWARE_SECTION_KEY, section);
  } catch {
    /* ignore */
  }
});

const firmwareTopTabs = computed(() => [
  { key: "usb", label: t("flashing.tab_usb_flash") },
  { key: "ble_ota", label: t("flashing.tab_ble_ota") },
]);

const flashingSubTabs = computed(() => [
  { key: "config", label: t("flashing.tab_config") },
  { key: "status", label: t("flashing.tab_status") },
]);

onMounted(() => {
  try {
    const saved = localStorage.getItem(FIRMWARE_SECTION_KEY);
    if (saved === "usb" || saved === "ble_ota") {
      firmwareSection.value = saved;
    }
  } catch {
    /* ignore */
  }
});

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
const headerDescription = computed(() =>
  firmwareSection.value === "ble_ota" ? t("flashing.description_ble_ota") : t("flashing.description"),
);
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

const canStartFromConfig = computed(() => {
  if (props.mode === "manual") return canStartManual.value;
  if (props.mode === "auto_slot") return canToggleAuto.value;
  return canStartBatch.value;
});

/** 与 FirmwareExecutionCard 主按钮文案一致 */
const configStartButtonLabel = computed(() => {
  if (props.busy) return t("flashing.busy");
  if (props.mode === "manual") return t("flashing.start");
  if (props.mode === "auto_slot") {
    return props.autoUpdateEnabled ? t("flashing.disable_auto") : t("flashing.enable_auto");
  }
  return t("flashing.start_batch");
});

async function startFromConfig(): Promise<void> {
  activeTab.value = "status";
  await nextTick();
  if (props.mode === "manual") {
    emit("startFlash");
  } else if (props.mode === "auto_slot") {
    emit("toggleAutoUpdate");
  } else {
    emit("startBatchFlash");
  }
}
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
    <PageHeader
      :title="t('flashing.title')"
      :description="headerDescription"
      :logo-src="logoUrl"
      logo-alt="FoxDock Logo"
    />

    <BaseTabs
      v-model="firmwareSection"
      :tabs="firmwareTopTabs"
      :aria-label="t('flashing.firmware_top_tabs_aria')"
    />

    <div v-show="firmwareSection === 'ble_ota'" class="ble-ota-wrap">
      <BleOtaPanel />
    </div>

    <div v-show="firmwareSection === 'usb'" class="usb-firmware-wrap">
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
          <BaseButton
            variant="outline"
            :aria-expanded="isConnectionExpanded"
            aria-controls="flashing-connection-panel"
            @click="toggleConnectionPanel"
          >
            {{
              isConnectionExpanded
                ? t("home.connection_bar_collapse")
                : t("home.connection_bar_expand")
            }}
          </BaseButton>
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

    <BaseTabs
      v-model="activeTab"
      :tabs="flashingSubTabs"
      :aria-label="t('flashing.tabs_aria_label')"
    />

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

        <div class="config-start-action">
          <p class="config-start-action__title">{{ t("flashing.start_from_config") }}</p>
          <p v-if="showPrimaryBlockHint" class="config-start-hint" role="status">
            {{ primaryBlockHint }}
          </p>
          <BaseButton
            class="config-start-action__btn"
            :disabled="controlsLocked || !canStartFromConfig"
            @click="startFromConfig"
          >
            {{ configStartButtonLabel }}
          </BaseButton>
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
  </div>
</template>

<style scoped>
.flashing-view {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
}

.ble-ota-wrap,
.usb-firmware-wrap {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
  min-height: 0;
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

.config-start-action {
  margin-top: var(--spacing-md);
  padding-top: var(--spacing-md);
  border-top: var(--border-width) solid var(--color-secondary-hover);
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
}

.config-start-action__title {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text-secondary);
}

.config-start-hint {
  margin: 0;
  font-size: 12px;
  line-height: 1.45;
  color: var(--color-text-light);
}

.config-start-action__btn {
  align-self: flex-start;
}
</style>
