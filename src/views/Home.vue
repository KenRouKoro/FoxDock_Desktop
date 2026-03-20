<script setup lang="ts">
import { useI18n } from "vue-i18n";
import ConnectionPanel from "../components/ConnectionPanel.vue";
import TrackerStatusComponent from "../components/TrackerStatus.vue";
import TrackerControl from "../components/TrackerControl.vue";
import logoUrl from "../assets/FoxApplication.png";
import BaseButton from "../components/ui/BaseButton.vue";

// 类型定义
type DockPort = {
  portName: string;
  displayName: string;
  serialNumber: string | null;
};

type DockInfo = {
  project: string;
  version: string;
  mcu: string;
  extra?: Record<string, unknown>;
};

type TrackerStatus = {
  id: number;
  inserted: boolean;
  usbPath?: string;
};

const { t, locale } = useI18n();

defineProps<{
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
  (e: 'refreshDocks'): void;
  (e: 'connectDock', portName: string): void;
  (e: 'disconnectDock'): void;
  (e: 'runSingleAction', action: string, trackerId: number): void;
  (e: 'runAllAction', action: string): void;
  (e: 'toggleLed'): void;
  (e: 'refreshStatus'): void;
  (e: 'setBlMode', mode: number): void;
  (e: 'setAutoSleep', enabled: boolean): void;
  (e: 'toggleLocale'): void;
  (e: 'openDebug'): void;
}>();
</script>

<template>
  <div class="home-view">
    <header class="header">
      <div class="header-bar">
        <div class="header-title-area">
          <img :src="logoUrl" class="logo" alt="FoxDock Logo" />
          <div class="header-text">
            <h1>{{ t('app.title') }}</h1>
            <p>{{ t('app.subtitle') }}</p>
          </div>
        </div>
        <div class="header-actions">
          <BaseButton variant="outline" @click="emit('toggleLocale')">
            {{ locale === 'zh' ? 'EN' : '中' }}
          </BaseButton>
          <BaseButton variant="debug" @click="emit('openDebug')">
            {{ t('app.debug_btn') }}
          </BaseButton>
        </div>
      </div>
    </header>

    <ConnectionPanel 
      :docks="docks"
      :connected-port-name="connectedPortName"
      :dock-info="dockInfo"
      :loading="loading"
      @refresh="emit('refreshDocks')"
      @connect="(port) => emit('connectDock', port)"
      @disconnect="emit('disconnectDock')"
    />

    <TrackerStatusComponent 
      :trackers="trackers" 
      :disabled="loading || !connectedPortName"
      @run-single-action="(action, id) => emit('runSingleAction', action, id)"
    />

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
</template>

<style scoped>
.home-view {
  display: flex;
  flex-direction: column;
  gap: 0; /* Components already have margins or we handle it in main-content */
}

.header {
  margin-bottom: var(--spacing-md);
  border: var(--border-width) solid var(--color-secondary);
  background: var(--color-bg-header);
  padding: var(--spacing-sm) var(--spacing-md);
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
  height: 32px;
  width: auto;
}

.header-text h1 {
  margin: 0;
  font-size: 16px;
  line-height: 1.2;
}

.header-text p {
  margin: 0;
  font-size: 11px;
  color: var(--color-text-light);
}

.header-actions {
  display: flex;
  gap: var(--spacing-xs);
}
</style>
