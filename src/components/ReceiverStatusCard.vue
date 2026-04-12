<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { listen } from "@tauri-apps/api/event";
import BasePanel from "./ui/BasePanel.vue";
import ReceiverHidParamModal from "./ReceiverHidParamModal.vue";
import type {
  ReceiverStatus,
  SerialConsoleLocalCommand,
  SerialConsoleTargetHint,
} from "../types/serialConsole";
import type { OpenContextMenuWindowRequest } from "../types/contextMenu";
import { findReceiverLocalCommandByKey } from "../utils/serialConsoleCommands";
import { estimateReceiverParentMenuSize } from "../utils/contextMenu";
import { parseTrackerVersion } from "../utils/trackerVersion";

const { t } = useI18n();

const props = defineProps<{
  receiverStatus: ReceiverStatus;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  (e: "openSerialConsole", targetHint: SerialConsoleTargetHint): void;
  (e: "openContextMenu", req: OpenContextMenuWindowRequest): void;
  (e: "runReceiverHidLine", line: string): void;
}>();

const paramModalOpen = ref(false);
const paramModalCommand = ref<SerialConsoleLocalCommand | null>(null);

let unlistenReceiverParam: (() => void) | undefined;

function showContextMenu(event: MouseEvent) {
  event.preventDefault();
  const canUse = props.receiverStatus.inserted && !props.disabled;
  const est = estimateReceiverParentMenuSize();
  emit("openContextMenu", {
    screenX: event.screenX,
    screenY: event.screenY,
    width: est.width,
    height: est.height,
    payload: {
      variant: "receiver",
      canUseReceiverActions: canUse,
    },
  });
}

function openParamModal(cmd: SerialConsoleLocalCommand): void {
  if (!cmd.parametric) return;
  paramModalCommand.value = cmd;
  paramModalOpen.value = true;
}

function openParamModalFromKey(cmdKey: string): void {
  const cmd = findReceiverLocalCommandByKey(cmdKey);
  if (!cmd?.parametric) return;
  openParamModal(cmd);
}

function onParamSubmit(line: string): void {
  emit("runReceiverHidLine", line);
  paramModalCommand.value = null;
}

onMounted(async () => {
  unlistenReceiverParam = await listen<{ cmdKey: string }>(
    "receiver-context-param-open",
    (event) => {
      const key = event.payload?.cmdKey;
      if (key) openParamModalFromKey(key);
    },
  );
});

onUnmounted(() => {
  unlistenReceiverParam?.();
});
</script>

<template>
  <BasePanel :title="t('tracker_status.receiver_panel_title')">
    <div
      class="receiver-cell"
      :class="{ inserted: receiverStatus.inserted }"
      @contextmenu="showContextMenu"
    >
      <div class="tracker-info tracker-info--split">
        <div class="slot-identity-block">
          <span class="slot-name">{{ t("tracker_status.receiver") }}</span>
          <span
            v-if="receiverStatus.displayName || receiverStatus.portName"
            class="usb-path"
          >
            {{ receiverStatus.displayName || receiverStatus.portName }}
          </span>
        </div>
        <div v-if="receiverStatus.receiverVersion" class="tracker-version-block">
          <template
            v-for="vd in [parseTrackerVersion(receiverStatus.receiverVersion)]"
            :key="'rv'"
          >
            <span class="tracker-version-line">{{ vd.mainLine }}</span>
            <span v-if="vd.hashLine" class="tracker-version-hash">{{ vd.hashLine }}</span>
          </template>
        </div>
      </div>
      <span class="tracker-cell-status">
        {{
          receiverStatus.inserted
            ? t("tracker_status.inserted")
            : t("tracker_status.not_inserted")
        }}
      </span>
    </div>

    <ReceiverHidParamModal
      v-model:open="paramModalOpen"
      variant="local"
      :local-command="paramModalCommand"
      @submit="onParamSubmit"
    />
  </BasePanel>
</template>

<style scoped>
.receiver-cell {
  border: var(--border-width) solid var(--color-secondary);
  background: var(--color-bg-slot-idle);
  padding: var(--spacing-sm);
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--spacing-sm);
  cursor: default;
  user-select: none;
  border-style: dashed;
}

.tracker-cell-status {
  flex-shrink: 1;
  min-width: 0;
  max-width: 48%;
  text-align: right;
  line-height: 1.35;
  word-break: break-word;
}

.tracker-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  flex: 1;
}

.tracker-info--split {
  display: grid;
  grid-template-columns: minmax(84px, auto) minmax(0, 1fr);
  align-items: start;
  gap: var(--spacing-sm);
}

.slot-name {
  font-weight: bold;
  flex-shrink: 0;
  line-height: 1.35;
}

.slot-identity-block {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 1px;
  min-width: 0;
}

.usb-path {
  font-size: 11px;
  color: var(--color-secondary);
  font-family: var(--font-family-mono);
  line-height: 1.3;
}

.tracker-version-block {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 1px;
  min-width: 0;
  flex: 1;
  font-size: 12px;
  line-height: 1.35;
  color: var(--color-text-light);
  font-family: var(--font-family-mono);
}

.tracker-version-line {
  overflow-wrap: anywhere;
  word-break: break-word;
  line-height: inherit;
}

.tracker-version-hash {
  display: block;
  overflow-wrap: anywhere;
  word-break: break-all;
  line-height: 1.35;
}

.receiver-cell.inserted .usb-path {
  color: var(--color-success-border);
}

.receiver-cell.inserted .tracker-version-block {
  color: var(--color-text-main);
}

.receiver-cell.inserted {
  border-color: var(--color-success-border);
  background: var(--color-success-bg);
}

.receiver-cell:hover {
  background: var(--color-secondary-hover);
}

.receiver-cell.inserted:hover {
  background: var(--color-success-hover);
}
</style>
