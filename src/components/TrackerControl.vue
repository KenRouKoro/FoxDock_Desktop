<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import BaseButton from "./ui/BaseButton.vue";
import BasePanel from "./ui/BasePanel.vue";
import BaseSelect from "./ui/BaseSelect.vue";

const { t } = useI18n();

const props = defineProps<{
  connectedPortName: string;
  loading: boolean;
  ledEnabled: boolean;
  blMode: number | null;
  blModeName: string;
  autoSleepEnabled: boolean;
  blModeOptions: Array<{ mode: number; label: string }>;
}>();

const emit = defineEmits<{
  (e: 'runSingleAction', action: string, id: number): void;
  (e: 'runAllAction', action: string): void;
  (e: 'toggleLed'): void;
  (e: 'refreshStatus'): void;
  (e: 'setBlMode', mode: number): void;
  (e: 'setAutoSleep', enabled: boolean): void;
}>();

const selectedTrackerId = ref(1);
const selectedBlMode = ref(0);

const singleActions = [
  { label: "actions.ret", value: "ret" },
  { label: "actions.bl", value: "bl" },
  { label: "actions.wake_up", value: "wake_up" },
  { label: "actions.sleep", value: "sleep" },
  { label: "actions.pair", value: "pair" },
];

const allActions = [
  { label: "actions.ret_all", value: "ret_all" },
  { label: "actions.bl_all", value: "bl_all" },
  { label: "actions.wake_up_all", value: "wake_up_all" },
  { label: "actions.sleep_all", value: "sleep_all" },
  { label: "actions.pair_all", value: "pair_all" },
];

watch(
  () => props.blMode,
  (value) => {
    if (typeof value === "number") {
      selectedBlMode.value = value;
    }
  },
  { immediate: true },
);
</script>

<template>
  <BasePanel :title="t('tracker_control.title')">
    <section class="section-block">
      <h3 class="section-title">{{ t('tracker_control.single_section') }}</h3>
      <div class="row">
        <label for="trackerId">{{ t('tracker_control.slot_label') }}</label>
        <BaseSelect id="trackerId" v-model.number="selectedTrackerId" :disabled="loading">
          <option v-for="id in 10" :key="id" :value="id">{{ id }}</option>
        </BaseSelect>
      </div>
      <div class="button-grid">
        <BaseButton
          v-for="item in singleActions"
          :key="item.value"
          variant="outline"
          :disabled="loading || !connectedPortName"
          @click="emit('runSingleAction', item.value, selectedTrackerId)"
        >
          {{ t(`tracker_control.${item.label}`) }}
        </BaseButton>
      </div>
    </section>
    <section class="section-block">
      <h3 class="section-title">{{ t('tracker_control.all_section') }}</h3>
      <div class="button-grid">
        <BaseButton
          v-for="item in allActions"
          :key="item.value"
          variant="outline"
          :disabled="loading || !connectedPortName"
          @click="emit('runAllAction', item.value)"
        >
          {{ t(`tracker_control.${item.label}`) }}
        </BaseButton>
      </div>
    </section>
    <section class="section-block section-block-last">
      <h3 class="section-title">{{ t('tracker_control.dock_section') }}</h3>
      <div class="row">
        <BaseButton variant="outline" :disabled="loading || !connectedPortName" @click="emit('toggleLed')">
          {{ t('tracker_control.led_label') }}{{ ledEnabled ? t('common.on') : t('common.off') }}
        </BaseButton>
        <BaseButton
          variant="outline"
          :disabled="loading || !connectedPortName"
          @click="emit('setAutoSleep', !autoSleepEnabled)"
        >
          {{ t('tracker_control.auto_sleep_label') }}{{ autoSleepEnabled ? t('common.on') : t('common.off') }}
        </BaseButton>
        <BaseButton variant="outline" :disabled="loading || !connectedPortName" @click="emit('refreshStatus')">{{ t('tracker_control.refresh_status') }}</BaseButton>
      </div>
      <div class="row row-end">
        <label for="blMode">{{ t('tracker_control.bl_mode_label') }}</label>
        <BaseSelect
          id="blMode"
          v-model.number="selectedBlMode"
          :disabled="loading || !connectedPortName"
        >
          <option
            v-for="item in blModeOptions"
            :key="item.mode"
            :value="item.mode"
          >
            {{ item.mode }} - {{ item.label }}
          </option>
        </BaseSelect>
        <BaseButton
          variant="outline"
          :disabled="loading || !connectedPortName"
          @click="emit('setBlMode', selectedBlMode)"
        >
          {{ t('tracker_control.set_bl_mode') }}
        </BaseButton>
        <span class="bl-mode-current">
          {{ t('tracker_control.current_bl_mode', { mode: blMode ?? '-', name: blModeName || '-' }) }}
        </span>
      </div>
    </section>
  </BasePanel>
</template>

<style scoped>
.section-block {
  margin-bottom: var(--spacing-md);
  padding-bottom: var(--spacing-md);
  border-bottom: var(--border-width) solid var(--color-secondary-hover);
}

.section-block-last {
  margin-bottom: 0;
  padding-bottom: 0;
  border-bottom: none;
}

.section-title {
  margin: 0 0 var(--spacing-sm);
  font-size: 14px;
  font-weight: 600;
  color: var(--color-text-secondary);
}

.row {
  display: flex;
  gap: var(--spacing-sm);
  margin-bottom: var(--spacing-sm);
  align-items: center;
  flex-wrap: wrap;
}

.row-end {
  margin-bottom: 0;
}

.button-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: var(--spacing-sm);
  margin-bottom: 0;
}

.bl-mode-current {
  font-size: 12px;
  color: var(--color-text-light);
}
</style>
