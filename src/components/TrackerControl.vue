<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import BaseButton from "./ui/BaseButton.vue";
import BasePanel from "./ui/BasePanel.vue";
import BaseSelect from "./ui/BaseSelect.vue";

const { t } = useI18n();

defineProps<{
  connectedPortName: string;
  loading: boolean;
  ledEnabled: boolean;
}>();

const emit = defineEmits<{
  (e: 'runSingleAction', action: string, id: number): void;
  (e: 'runAllAction', action: string): void;
  (e: 'toggleLed'): void;
  (e: 'refreshStatus'): void;
}>();

const selectedTrackerId = ref(1);

const singleActions = [
  { label: "actions.ret", value: "ret" },
  { label: "actions.bl", value: "bl" },
  { label: "actions.sleep", value: "sleep" },
  { label: "actions.pair", value: "pair" },
];

const allActions = [
  { label: "actions.ret_all", value: "ret_all" },
  { label: "actions.bl_all", value: "bl_all" },
  { label: "actions.sleep_all", value: "sleep_all" },
  { label: "actions.pair_all", value: "pair_all" },
];
</script>

<template>
  <BasePanel :title="t('tracker_control.title')">
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
    <div class="row">
      <BaseButton variant="outline" :disabled="loading || !connectedPortName" @click="emit('toggleLed')">
        {{ t('tracker_control.led_label') }}{{ ledEnabled ? t('common.on') : t('common.off') }}
      </BaseButton>
      <BaseButton variant="outline" :disabled="loading || !connectedPortName" @click="emit('refreshStatus')">{{ t('tracker_control.refresh_status') }}</BaseButton>
    </div>
  </BasePanel>
</template>

<style scoped>
.row {
  display: flex;
  gap: var(--spacing-sm);
  margin-bottom: var(--spacing-sm);
  align-items: center;
  flex-wrap: wrap;
}

.button-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: var(--spacing-sm);
  margin-bottom: var(--spacing-sm);
}
</style>
