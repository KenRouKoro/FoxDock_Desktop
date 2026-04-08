<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import ConnectionPanel from "../components/ConnectionPanel.vue";
import TrackerStatusComponent from "../components/TrackerStatus.vue";
import TrackerControl from "../components/TrackerControl.vue";
import BaseTabs from "../components/ui/BaseTabs.vue";
import PageHeader from "../components/ui/PageHeader.vue";
import logoUrl from "../assets/FoxApplication.png";
import type { DockInfo, DockPort, TrackerStatus } from "../types/dock";
import type {
  ReceiverStatus,
  SerialConsoleTargetHint,
} from "../types/serialConsole";

const { t } = useI18n();

const props = defineProps<{
  docks: DockPort[];
  connectedPortName: string;
  dockInfo: DockInfo | null;
  trackers: TrackerStatus[];
  receiverStatus: ReceiverStatus;
  ledEnabled: boolean;
  loading: boolean;
  blMode: number | null;
  blModeName: string;
  autoSleepEnabled: boolean;
  blModeOptions: Array<{ mode: number; label: string }>;
}>();

const emit = defineEmits<{
  (e: "refreshDocks"): void;
  (e: "connectDock", portName: string): void;
  (e: "disconnectDock"): void;
  (e: "runSingleAction", action: string, trackerId: number): void;
  (e: "runAllAction", action: string): void;
  (e: "toggleLed"): void;
  (e: "refreshStatus"): void;
  (e: "setBlMode", mode: number): void;
  (e: "setAutoSleep", enabled: boolean): void;
  (e: "openSerialConsole", targetHint: SerialConsoleTargetHint): void;
}>();

type HomeTab = "trackers" | "control";

const activeTab = ref<HomeTab>("trackers");
const isConnectionPanelExpanded = ref(true);

const insertedCount = computed(
  () => props.trackers.filter((item) => item.inserted).length,
);

watch(
  () => props.connectedPortName,
  (name, oldName) => {
    if (!name) {
      isConnectionPanelExpanded.value = true;
    } else if (oldName === undefined || oldName === "") {
      isConnectionPanelExpanded.value = false;
      activeTab.value = "trackers";
    }
  },
  { immediate: true },
);

function toggleConnectionPanel() {
  isConnectionPanelExpanded.value = !isConnectionPanelExpanded.value;
}

const homeTabs = computed(() => [
  { key: "trackers", label: t("home.tab_trackers") },
  { key: "control", label: t("home.tab_control") },
]);
</script>

<template>
  <div class="home-view">
    <PageHeader
      :title="t('app.title')"
      :description="t('app.subtitle')"
      :logo-src="logoUrl"
      logo-alt="FoxDock Logo"
    />

    <section
      class="connection-bar"
      :class="{
        'connection-bar--ok': !!connectedPortName,
        'connection-bar--expanded': isConnectionPanelExpanded,
      }"
      :aria-label="t('home.connection_bar_label')"
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
            {{ t("home.summary_inserted", { count: insertedCount }) }}
          </span>
        </div>
        <button
          type="button"
          class="connection-bar__toggle"
          :aria-expanded="isConnectionPanelExpanded"
          aria-controls="home-connection-panel"
          @click="toggleConnectionPanel"
        >
          {{
            isConnectionPanelExpanded
              ? t("home.connection_bar_collapse")
              : t("home.connection_bar_expand")
          }}
        </button>
      </div>

      <div
        class="connection-bar__panel-outer"
        :class="{ 'connection-bar__panel-outer--open': isConnectionPanelExpanded }"
        :aria-hidden="!isConnectionPanelExpanded"
      >
        <div class="connection-bar__panel-inner">
          <div id="home-connection-panel" class="connection-bar__panel">
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
      :tabs="homeTabs"
      :aria-label="t('home.tabs_aria_label')"
    />

    <div class="home-panels">
      <div v-show="activeTab === 'trackers'" role="tabpanel">
        <TrackerStatusComponent
          :trackers="trackers"
          :receiver-status="receiverStatus"
          :disabled="loading || !connectedPortName"
          @run-single-action="(action, id) => emit('runSingleAction', action, id)"
          @open-serial-console="(targetHint) => emit('openSerialConsole', targetHint)"
        />
      </div>
      <div v-show="activeTab === 'control'" role="tabpanel">
        <TrackerControl
          :connected-port-name="connectedPortName"
          :loading="loading"
          :led-enabled="ledEnabled"
          :bl-mode="blMode"
          :bl-mode-name="blModeName"
          :auto-sleep-enabled="autoSleepEnabled"
          :bl-mode-options="blModeOptions"
          @run-single-action="(action, id) => emit('runSingleAction', action, id)"
          @run-all-action="(action) => emit('runAllAction', action)"
          @toggle-led="emit('toggleLed')"
          @refresh-status="emit('refreshStatus')"
          @set-bl-mode="(mode) => emit('setBlMode', mode)"
          @set-auto-sleep="(enabled) => emit('setAutoSleep', enabled)"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.home-view {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
}

.home-panels {
  display: flex;
  flex-direction: column;
  gap: 0;
  min-height: 0;
}
</style>
