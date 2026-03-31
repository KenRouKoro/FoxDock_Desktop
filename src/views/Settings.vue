<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import BasePanel from "../components/ui/BasePanel.vue";

const { t } = useI18n();

type AppVersionInfo = {
  appName: string;
  appVersion: string;
};

const appVersionInfo = ref<AppVersionInfo | null>(null);

onMounted(async () => {
  try {
    appVersionInfo.value = await invoke<AppVersionInfo>("get_app_version");
  } catch (error) {
    console.error("Failed to load app version", error);
  }
});
</script>

<template>
  <div class="settings-page">
    <BasePanel :title="t('settings.title')">
      <div class="placeholder">
        <p>{{ t('settings.placeholder') }}</p>
      </div>
    </BasePanel>

    <BasePanel :title="t('app.info_title')">
      <div class="info-content">
        <div class="info-item">
          <span class="label">{{ t('app.app_name') }}</span>
          <span class="value">{{ appVersionInfo?.appName ?? "-" }}</span>
        </div>
        <div class="info-item">
          <span class="label">{{ t('app.app_version') }}</span>
          <span class="value">{{ appVersionInfo?.appVersion ?? "-" }}</span>
        </div>
      </div>
    </BasePanel>
  </div>
</template>

<style scoped>
.settings-page {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
  padding-bottom: var(--spacing-xl);
}

.placeholder {
  padding: var(--spacing-xl);
  text-align: center;
  color: var(--color-text-light);
  border: 1px dashed var(--color-secondary);
  background: var(--color-bg-panel);
}

.info-content {
  display: grid;
  gap: var(--spacing-sm);
}

.info-item {
  display: grid;
  grid-template-columns: 120px 1fr;
  gap: var(--spacing-sm);
  align-items: center;
  border: 1px solid var(--color-secondary-hover);
  background: var(--color-bg-panel);
  padding: var(--spacing-sm);
}

.info-item .label {
  color: var(--color-text-light);
}

.info-item .value {
  font-family: var(--font-family-mono);
  color: var(--color-text-main);
}
</style>
