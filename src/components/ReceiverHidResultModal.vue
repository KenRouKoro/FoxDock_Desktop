<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import BaseButton from "./ui/BaseButton.vue";
import ReceiverHidRssiScanPanel from "./ReceiverHidRssiScanPanel.vue";
import ReceiverHidStoredDevicesPanel from "./ReceiverHidStoredDevicesPanel.vue";
import type { RssiScanParsed } from "../utils/receiverHidRichMessage";

const props = defineProps<{
  open: boolean;
  variant: "rssi" | "list";
  truncated: boolean;
  rssiData: RssiScanParsed | null;
  listMacs: string[] | null;
  rawText: string;
}>();

const emit = defineEmits<{
  (e: "update:open", value: boolean): void;
}>();

const { t } = useI18n();

function close(): void {
  emit("update:open", false);
}

function onBackdropClick(event: MouseEvent): void {
  if ((event.target as HTMLElement).classList.contains("rhid-result-backdrop")) {
    close();
  }
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape" && props.open) {
    event.preventDefault();
    close();
  }
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
});

const modalTitle = () => {
  if (props.variant === "rssi") {
    return t("receiver_hid_rich.modal_title_rssi");
  }
  return t("receiver_hid_rich.modal_title_list");
};
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="rhid-result-backdrop"
      role="presentation"
      @click="onBackdropClick"
    >
      <div
        class="rhid-result-modal"
        role="dialog"
        aria-modal="true"
        :aria-label="modalTitle()"
        @click.stop
      >
        <div class="rhid-result-head">
          <h3 class="rhid-result-title">{{ modalTitle() }}</h3>
          <BaseButton type="button" variant="outline" @click="close">
            {{ t("receiver_hid_rich.modal_close") }}
          </BaseButton>
        </div>
        <p v-if="truncated" class="rhid-result-trunc">
          {{ t("receiver_hid_rich.truncated_warning") }}
        </p>
        <div class="rhid-result-body">
          <ReceiverHidRssiScanPanel v-if="variant === 'rssi' && rssiData" :data="rssiData" />
          <ReceiverHidStoredDevicesPanel
            v-else-if="variant === 'list' && listMacs"
            :macs="listMacs"
            :raw="rawText"
          />
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.rhid-result-backdrop {
  position: fixed;
  inset: 0;
  z-index: 9998;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-md);
}

.rhid-result-modal {
  width: min(640px, 100%);
  max-height: min(90vh, 720px);
  background: var(--color-bg-white);
  border: var(--border-width) solid var(--color-secondary);
  box-shadow: var(--box-shadow);
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.rhid-result-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-md);
  padding: var(--spacing-md) var(--spacing-lg);
  border-bottom: var(--border-width) solid var(--color-secondary);
  flex-shrink: 0;
}

.rhid-result-title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}

.rhid-result-trunc {
  margin: 0;
  padding: var(--spacing-sm) var(--spacing-lg) 0;
  font-size: 13px;
  color: var(--color-error);
  font-weight: 600;
}

.rhid-result-body {
  padding: var(--spacing-md) var(--spacing-lg) var(--spacing-lg);
  overflow: auto;
  min-height: 0;
}
</style>
