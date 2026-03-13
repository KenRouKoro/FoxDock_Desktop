<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
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
  <section class="panel">
    <h2>{{ t('tracker_control.title') }}</h2>
    <div class="row">
      <label for="trackerId">{{ t('tracker_control.slot_label') }}</label>
      <select id="trackerId" v-model.number="selectedTrackerId" :disabled="loading">
        <option v-for="id in 10" :key="id" :value="id">{{ id }}</option>
      </select>
    </div>
    <div class="button-grid">
      <button
        v-for="item in singleActions"
        :key="item.value"
        :disabled="loading || !connectedPortName"
        @click="emit('runSingleAction', item.value, selectedTrackerId)"
      >
        {{ t(`tracker_control.${item.label}`) }}
      </button>
    </div>
    <div class="button-grid">
      <button
        v-for="item in allActions"
        :key="item.value"
        :disabled="loading || !connectedPortName"
        @click="emit('runAllAction', item.value)"
      >
        {{ t(`tracker_control.${item.label}`) }}
      </button>
    </div>
    <div class="row">
      <button :disabled="loading || !connectedPortName" @click="emit('toggleLed')">
        {{ t('tracker_control.led_label') }}{{ ledEnabled ? t('common.on') : t('common.off') }}
      </button>
      <button :disabled="loading || !connectedPortName" @click="emit('refreshStatus')">{{ t('tracker_control.refresh_status') }}</button>
    </div>
  </section>
</template>

<style scoped>
.panel {
  border: 2px solid #59a9ff;
  background: #f4faff;
  padding: 12px;
  margin-bottom: 12px;
}

.panel h2 {
  margin: 0 0 10px;
  font-size: 18px;
  color: #184470;
}

.row {
  display: flex;
  gap: 8px;
  margin-bottom: 10px;
  align-items: center;
  flex-wrap: wrap;
}

.button-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 8px;
  margin-bottom: 10px;
}

select,
button {
  border: 2px solid #59a9ff;
  border-radius: 0;
  background: #ffffff;
  color: #12304f;
  padding: 6px 10px;
  font-size: 14px;
}

button {
  cursor: pointer;
}

button:hover:enabled {
  background: #d7ebff;
}

button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
</style>
