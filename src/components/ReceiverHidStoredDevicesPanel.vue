<script setup lang="ts">
import { useI18n } from "vue-i18n";
import BaseButton from "./ui/BaseButton.vue";

const props = defineProps<{
  macs: string[];
  raw: string;
}>();

const { t } = useI18n();

async function copyRaw(): Promise<void> {
  try {
    await navigator.clipboard.writeText(props.raw);
  } catch {
    /* ignore */
  }
}

async function copyOne(mac: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(mac);
  } catch {
    /* ignore */
  }
}
</script>

<template>
  <div class="list-panel">
    <p class="list-hint">{{ t("receiver_hid_rich.list_count", { count: macs.length }) }}</p>
    <ul class="mac-list">
      <li v-for="(mac, i) in macs" :key="`${mac}-${i}`" class="mac-row">
        <span class="mac-index mono">{{ i }}</span>
        <code class="mac-code mono">{{ mac }}</code>
        <button
          type="button"
          class="mac-copy"
          :title="t('receiver_hid_rich.copy_one')"
          @click="copyOne(mac)"
        >
          {{ t("receiver_hid_rich.copy_one_short") }}
        </button>
      </li>
    </ul>
    <div class="list-actions">
      <BaseButton type="button" variant="outline" @click="copyRaw">
        {{ t("receiver_hid_rich.copy_raw") }}
      </BaseButton>
    </div>
  </div>
</template>

<style scoped>
.list-panel {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
  min-height: 0;
}

.list-hint {
  margin: 0;
  font-size: 14px;
  color: var(--color-text-muted);
}

.mac-list {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: min(52vh, 420px);
  overflow: auto;
  border: var(--border-width) solid var(--color-secondary);
  border-radius: 2px;
}

.mac-row {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-xs) var(--spacing-sm);
  border-bottom: 1px solid var(--color-secondary);
  font-size: 13px;
}

.mac-row:last-child {
  border-bottom: none;
}

.mono {
  font-family: ui-monospace, monospace;
}

.mac-index {
  flex-shrink: 0;
  min-width: 2.25rem;
  text-align: right;
  color: var(--color-text-muted);
  user-select: none;
}

.mac-code {
  flex: 1;
  min-width: 0;
  font-size: 13px;
}

.mac-copy {
  flex-shrink: 0;
  margin-left: auto;
  padding: 2px 8px;
  font-size: 12px;
  border: var(--border-width) solid var(--color-secondary);
  background: var(--color-bg-white);
  border-radius: 2px;
  cursor: pointer;
  color: var(--color-text-main);
}

.mac-copy:hover {
  background: var(--color-bg-info-soft);
}

.list-actions {
  display: flex;
  justify-content: flex-end;
  padding-top: var(--spacing-xs);
}
</style>
