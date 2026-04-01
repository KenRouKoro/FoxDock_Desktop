<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import BasePanel from "../components/ui/BasePanel.vue";
import BaseSelect from "../components/ui/BaseSelect.vue";
import BaseButton from "../components/ui/BaseButton.vue";
import type { LanguagePreference } from "../types/settings";

const { t } = useI18n();

type AppVersionInfo = {
  appName: string;
  appVersion: string;
};

const props = defineProps<{
  languagePreference: LanguagePreference;
  debugEnabled: boolean;
}>();

const emit = defineEmits<{
  (e: "update:languagePreference", value: LanguagePreference): void;
  (e: "update:debugEnabled", value: boolean): void;
  (e: "openDebug"): void;
}>();

const appVersionInfo = ref<AppVersionInfo | null>(null);

onMounted(async () => {
  try {
    appVersionInfo.value = await invoke<AppVersionInfo>("get_app_version");
  } catch (error) {
    console.error("Failed to load app version", error);
  }
});

function onLanguageSelect(value: string) {
  emit("update:languagePreference", value as LanguagePreference);
}

function onDebugToggle(event: Event) {
  const checked = (event.target as HTMLInputElement).checked;
  emit("update:debugEnabled", checked);
}
</script>

<template>
  <div class="settings-page">
    <BasePanel :title="t('settings.system_title')">
      <div class="settings-rows">
        <div class="setting-row">
          <span class="setting-label">{{ t("settings.language_label") }}</span>
          <BaseSelect
            class="setting-control"
            :model-value="props.languagePreference"
            @update:model-value="onLanguageSelect"
          >
            <option value="system">{{ t("settings.language_option_system") }}</option>
            <option value="zh">{{ t("settings.language_option_zh") }}</option>
            <option value="en">{{ t("settings.language_option_en") }}</option>
          </BaseSelect>
        </div>

        <div class="setting-row setting-row--wrap">
          <div class="setting-label-block">
            <span class="setting-label">{{ t("settings.debug_section_title") }}</span>
            <p class="setting-hint">{{ t("settings.debug_section_hint") }}</p>
          </div>
          <div class="setting-debug-actions">
            <label class="checkbox-row">
              <input
                type="checkbox"
                class="checkbox-input"
                :checked="props.debugEnabled"
                @change="onDebugToggle"
              />
              <span>{{ t("settings.debug_enabled_label") }}</span>
            </label>
            <BaseButton variant="debug" @click="emit('openDebug')">
              {{ t("settings.open_debug_window") }}
            </BaseButton>
          </div>
        </div>
      </div>
    </BasePanel>

    <BasePanel :title="t('app.info_title')">
      <div class="info-content">
        <div class="info-item">
          <span class="label">{{ t("app.app_name") }}</span>
          <span class="value">{{ appVersionInfo?.appName ?? "-" }}</span>
        </div>
        <div class="info-item">
          <span class="label">{{ t("app.app_version") }}</span>
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

.settings-rows {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
}

.setting-row {
  display: grid;
  grid-template-columns: 120px 1fr;
  gap: var(--spacing-sm);
  align-items: center;
  border: var(--border-width-subtle) solid var(--color-secondary-hover);
  background: var(--color-bg-panel);
  padding: var(--spacing-sm);
}

.setting-row--wrap {
  grid-template-columns: 1fr;
  align-items: start;
}

.setting-label {
  color: var(--color-text-light);
  font-size: 14px;
}

.setting-label-block {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}

.setting-hint {
  margin: 0;
  font-size: 12px;
  color: var(--color-text-light);
  line-height: 1.4;
}

.setting-control {
  width: 100%;
  max-width: 280px;
  justify-self: end;
}

.setting-debug-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--spacing-md);
}

.checkbox-row {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-xs);
  cursor: pointer;
  font-size: 14px;
  color: var(--color-text-main);
  user-select: none;
}

.checkbox-input {
  width: 16px;
  height: 16px;
  accent-color: var(--color-primary);
  cursor: pointer;
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
  border: var(--border-width-subtle) solid var(--color-secondary-hover);
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
