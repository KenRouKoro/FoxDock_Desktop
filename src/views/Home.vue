<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import ConnectionPanel from "../components/ConnectionPanel.vue";
import TrackerStatusComponent from "../components/TrackerStatus.vue";
import TrackerControl from "../components/TrackerControl.vue";
import logoUrl from "../assets/FoxApplication.png";
import type { DockInfo, DockPort, TrackerStatus } from "../types/dock";

const { t } = useI18n();

const props = defineProps<{
  docks: DockPort[];
  connectedPortName: string;
  dockInfo: DockInfo | null;
  trackers: TrackerStatus[];
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

function setTab(tab: HomeTab) {
  activeTab.value = tab;
}

function toggleConnectionPanel() {
  isConnectionPanelExpanded.value = !isConnectionPanelExpanded.value;
}
</script>

<template>
  <div class="home-view">
    <header class="header">
      <div class="header-bar">
        <div class="header-title-area">
          <img :src="logoUrl" class="logo" alt="FoxDock Logo" />
          <div class="header-text">
            <h1>{{ t("app.title") }}</h1>
            <p>{{ t("app.subtitle") }}</p>
          </div>
        </div>
      </div>
    </header>

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

    <nav class="home-tabs" role="tablist" :aria-label="t('home.tabs_aria_label')">
      <button
        type="button"
        role="tab"
        class="home-tab"
        :class="{ 'home-tab--active': activeTab === 'trackers' }"
        :aria-selected="activeTab === 'trackers'"
        @click="setTab('trackers')"
      >
        {{ t("home.tab_trackers") }}
      </button>
      <button
        type="button"
        role="tab"
        class="home-tab"
        :class="{ 'home-tab--active': activeTab === 'control' }"
        :aria-selected="activeTab === 'control'"
        @click="setTab('control')"
      >
        {{ t("home.tab_control") }}
      </button>
    </nav>

    <div class="home-panels">
      <div v-show="activeTab === 'trackers'" role="tabpanel">
        <TrackerStatusComponent
          :trackers="trackers"
          :disabled="loading || !connectedPortName"
          @run-single-action="(action, id) => emit('runSingleAction', action, id)"
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

.connection-bar__toggle {
  flex-shrink: 0;
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

.mono {
  font-family: var(--font-family-mono);
  word-break: break-all;
}

.home-tabs {
  display: flex;
  border: var(--border-width) solid var(--color-secondary);
  background: var(--color-bg-header);
}

.home-tab {
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

.home-tab:last-child {
  border-right: none;
}

.home-tab:hover {
  background: var(--color-secondary-hover);
  color: var(--color-primary);
}

.home-tab--active {
  background: var(--color-bg-white);
  color: var(--color-primary);
  box-shadow: inset 0 -3px 0 var(--color-primary);
}

.home-panels {
  display: flex;
  flex-direction: column;
  gap: 0;
  min-height: 0;
}
</style>
