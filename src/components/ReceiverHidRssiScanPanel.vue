<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import BaseButton from "./ui/BaseButton.vue";
import type { RssiScanParsed } from "../utils/receiverHidRichMessage";
import { rssiDbmHeatRatio } from "../utils/receiverHidRichMessage";

const props = defineProps<{
  data: RssiScanParsed;
}>();

const { t } = useI18n();

const dbmRange = computed(() => {
  const xs = props.data.channels.map((c) => c.dbm);
  return {
    min: Math.min(...xs),
    max: Math.max(...xs),
  };
});

function cellStyle(dbm: number): Record<string, string> {
  const r = rssiDbmHeatRatio(dbm, dbmRange.value.min, dbmRange.value.max);
  const hue = 120 - r * 100;
  const light = 88 - r * 18;
  return {
    backgroundColor: `hsl(${hue} 42% ${light}%)`,
  };
}

async function copyRaw(): Promise<void> {
  try {
    await navigator.clipboard.writeText(props.data.raw);
  } catch {
    /* ignore */
  }
}
</script>

<template>
  <div class="rssi-panel">
    <p class="rssi-summary mono">{{ data.scanSummary }}</p>
    <dl class="rssi-meta">
      <div v-if="data.currentChannel != null" class="rssi-meta-row">
        <dt>{{ t("receiver_hid_rich.rssi_current_channel") }}</dt>
        <dd>{{ data.currentChannel }}</dd>
      </div>
      <div
        v-if="data.recommendedChannel != null && data.recommendedDbm != null"
        class="rssi-meta-row rssi-meta-row--rec"
      >
        <dt>{{ t("receiver_hid_rich.rssi_recommended") }}</dt>
        <dd>
          {{ t("receiver_hid_rich.rssi_recommended_value", {
            ch: data.recommendedChannel,
            dbm: data.recommendedDbm,
          }) }}
        </dd>
      </div>
    </dl>
    <p class="rssi-legend">{{ t("receiver_hid_rich.rssi_legend") }}</p>
    <div class="rssi-table-wrap">
      <table class="rssi-table">
        <thead>
          <tr>
            <th scope="col">{{ t("receiver_hid_rich.rssi_col_channel") }}</th>
            <th scope="col">{{ t("receiver_hid_rich.rssi_col_dbm") }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(row, i) in data.channels" :key="`${row.channel}-${i}`">
            <td class="mono">{{ row.channel }}</td>
            <td class="mono rssi-cell" :style="cellStyle(row.dbm)">{{ row.dbm }}</td>
          </tr>
        </tbody>
      </table>
    </div>
    <div class="rssi-actions">
      <BaseButton type="button" variant="outline" @click="copyRaw">
        {{ t("receiver_hid_rich.copy_raw") }}
      </BaseButton>
    </div>
  </div>
</template>

<style scoped>
.rssi-panel {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
  min-height: 0;
}

.mono {
  font-family: ui-monospace, monospace;
}

.rssi-summary {
  margin: 0;
  font-size: 13px;
  line-height: 1.45;
  color: var(--color-text-main);
  word-break: break-word;
}

.rssi-meta {
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}

.rssi-meta-row {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: var(--spacing-sm);
  font-size: 14px;
}

.rssi-meta-row dt {
  margin: 0;
  color: var(--color-text-muted);
  font-weight: 600;
}

.rssi-meta-row dd {
  margin: 0;
}

.rssi-meta-row--rec dd {
  font-weight: 600;
  color: var(--color-success);
}

.rssi-legend {
  margin: 0;
  font-size: 12px;
  color: var(--color-text-muted);
}

.rssi-table-wrap {
  max-height: min(52vh, 420px);
  overflow: auto;
  border: var(--border-width) solid var(--color-secondary);
  border-radius: 2px;
}

.rssi-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.rssi-table th,
.rssi-table td {
  padding: var(--spacing-xs) var(--spacing-sm);
  text-align: left;
  border-bottom: 1px solid var(--color-secondary);
}

.rssi-table thead th {
  position: sticky;
  top: 0;
  background: var(--color-bg-white);
  z-index: 1;
  font-weight: 600;
}

.rssi-cell {
  width: 40%;
}

.rssi-actions {
  display: flex;
  justify-content: flex-end;
  padding-top: var(--spacing-xs);
}
</style>
