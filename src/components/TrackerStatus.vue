<script setup lang="ts">
import { useI18n } from "vue-i18n";
import BasePanel from "./ui/BasePanel.vue";
import type { TrackerStatus } from "../types/dock";
import type { OpenContextMenuWindowRequest } from "../types/contextMenu";
import type { SerialConsoleTargetHint } from "../types/serialConsole";
import { estimateTrackerParentMenuSize } from "../utils/contextMenu";
import { parseTrackerVersion } from "../utils/trackerVersion";

const { t } = useI18n();

const props = defineProps<{
  trackers: TrackerStatus[];
  disabled?: boolean;
}>();

const emit = defineEmits<{
  (e: "runSingleAction", action: string, id: number): void;
  (e: "openSerialConsole", targetHint: SerialConsoleTargetHint): void;
  (e: "openContextMenu", req: OpenContextMenuWindowRequest): void;
}>();

const singleActions = [
  { label: "actions.ret", value: "ret" },
  { label: "actions.bl", value: "bl" },
  { label: "actions.wake_up", value: "wake_up" },
  { label: "actions.sleep", value: "sleep" },
  { label: "actions.pair", value: "pair" },
];

function showContextMenu(event: MouseEvent, item: TrackerStatus) {
  event.preventDefault();
  const est = estimateTrackerParentMenuSize(singleActions.length);
  emit("openContextMenu", {
    screenX: event.screenX,
    screenY: event.screenY,
    width: est.width,
    height: est.height,
    payload: {
      variant: "tracker",
      trackerId: item.id,
      inserted: item.inserted,
      actionsDisabled: Boolean(props.disabled),
    },
  });
}
</script>

<template>
  <BasePanel :title="t('tracker_status.title')">
    <div class="tracker-column">
      <div
        v-for="item in trackers"
        :key="item.id"
        class="tracker-cell"
        :class="{ inserted: item.inserted }"
        @contextmenu="showContextMenu($event, item)"
      >
        <div class="tracker-info tracker-info--split">
          <div class="slot-identity-block">
            <span class="slot-name">{{ t("tracker_status.slot", { id: item.id }) }}</span>
            <span v-if="item.usbPath" class="usb-path">{{ item.usbPath }}</span>
          </div>
          <div v-if="item.trackerVersion" class="tracker-version-block">
            <template v-for="vd in [parseTrackerVersion(item.trackerVersion)]" :key="`${item.id}-vd`">
              <span class="tracker-version-line">{{ vd.mainLine }}</span>
              <span v-if="vd.hashLine" class="tracker-version-hash">{{ vd.hashLine }}</span>
            </template>
          </div>
        </div>
        <span class="tracker-cell-status">{{
          item.inserted ? t("tracker_status.inserted") : t("tracker_status.not_inserted")
        }}</span>
      </div>
    </div>
  </BasePanel>
</template>

<style scoped>
.tracker-column {
  display: grid;
  grid-template-columns: 1fr;
  gap: var(--spacing-xs);
}

.tracker-cell {
  border: var(--border-width) solid var(--color-secondary);
  background: var(--color-bg-slot-idle);
  padding: var(--spacing-sm);
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--spacing-sm);
  cursor: default;
  user-select: none;
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

.slot-meta-row {
  display: flex;
  flex-direction: row;
  flex-wrap: wrap;
  align-items: flex-start;
  gap: var(--spacing-xs);
  column-gap: var(--spacing-sm);
  min-width: 0;
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

.tracker-cell.inserted .usb-path {
  color: var(--color-success-border);
}

.tracker-cell.inserted .tracker-version-block {
  color: var(--color-text-main);
}

.tracker-cell.inserted {
  border-color: var(--color-success-border);
  background: var(--color-success-bg);
}

.tracker-cell:hover {
  background: var(--color-secondary-hover);
}

.tracker-cell.inserted:hover {
  background: var(--color-success-hover);
}
</style>
