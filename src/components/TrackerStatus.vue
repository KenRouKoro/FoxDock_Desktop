<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import BasePanel from "./ui/BasePanel.vue";
import type { TrackerStatus } from "../types/dock";
import type {
  ReceiverStatus,
  SerialConsoleTargetHint,
} from "../types/serialConsole";

const { t } = useI18n();

const props = defineProps<{
  trackers: TrackerStatus[];
  receiverStatus: ReceiverStatus;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  (e: 'runSingleAction', action: string, id: number): void;
  (e: "openSerialConsole", targetHint: SerialConsoleTargetHint): void;
}>();

// --- 右键菜单逻辑 ---
const menuVisible = ref(false);
const menuPosition = ref({ x: 0, y: 0 });
const menuTarget = ref<
  | { kind: "tracker"; trackerId: number; inserted: boolean }
  | { kind: "receiver"; inserted: boolean }
  | null
>(null);

const showContextMenu = (event: MouseEvent, item: TrackerStatus) => {
  event.preventDefault();
  menuTarget.value = {
    kind: "tracker",
    trackerId: item.id,
    inserted: item.inserted,
  };
  menuPosition.value = { x: event.clientX, y: event.clientY };
  menuVisible.value = true;
};

const showReceiverContextMenu = (event: MouseEvent) => {
  event.preventDefault();
  menuTarget.value = {
    kind: "receiver",
    inserted: props.receiverStatus.inserted,
  };
  menuPosition.value = { x: event.clientX, y: event.clientY };
  menuVisible.value = true;
};

const closeMenu = () => {
  menuVisible.value = false;
};

const handleAction = (action: string) => {
  if (menuTarget.value?.kind === "tracker") {
    emit('runSingleAction', action, menuTarget.value.trackerId);
  }
  closeMenu();
};

const openConsole = () => {
  if (!menuTarget.value) return;
  if (menuTarget.value.kind === "receiver") {
    emit("openSerialConsole", { deviceType: "receiver" });
  } else {
    emit("openSerialConsole", {
      deviceType: "tracker",
      trackerId: menuTarget.value.trackerId,
    });
  }
  closeMenu();
};

const singleActions = [
  { label: "actions.ret", value: "ret" },
  { label: "actions.bl", value: "bl" },
  { label: "actions.wake_up", value: "wake_up" },
  { label: "actions.sleep", value: "sleep" },
  { label: "actions.pair", value: "pair" },
];

const canRunTrackerActions = computed(
  () =>
    menuTarget.value?.kind === "tracker" &&
    menuTarget.value.inserted &&
    !props.disabled,
);

const menuTitle = computed(() => {
  if (!menuTarget.value) return "";
  return menuTarget.value.kind === "receiver"
    ? t("tracker_status.receiver")
    : t("tracker_status.slot", { id: menuTarget.value.trackerId });
});

onMounted(() => {
  window.addEventListener('click', closeMenu);
});

onUnmounted(() => {
  window.removeEventListener('click', closeMenu);
});

/** 与固件 `format_usb_build_info_v1` 一致：`{major}.{minor}.{patch}+{tweak} {hash}`，可选末尾 ` dirty`。哈希放第二行展示。 */
function parseTrackerVersion(version: string): { mainLine: string; hashLine: string | null } {
  const dirtySuffix = version.endsWith(" dirty") ? " dirty" : "";
  const core = dirtySuffix ? version.slice(0, -" dirty".length) : version;
  const lastSpace = core.lastIndexOf(" ");
  if (lastSpace === -1) {
    return { mainLine: version, hashLine: null };
  }
  const tail = core.slice(lastSpace + 1);
  if (/^[0-9a-fA-F]{6,}$/.test(tail)) {
    return {
      mainLine: core.slice(0, lastSpace),
      hashLine: tail + dirtySuffix,
    };
  }
  return { mainLine: version, hashLine: null };
}
</script>

<template>
  <BasePanel :title="t('tracker_status.title')">
    <div class="tracker-column">
      <div
        class="tracker-cell receiver-cell"
        :class="{ inserted: receiverStatus.inserted }"
        @contextmenu="showReceiverContextMenu"
      >
        <div class="tracker-info">
          <div class="slot-meta-row">
            <span class="slot-name">{{ t("tracker_status.receiver") }}</span>
          </div>
          <span v-if="receiverStatus.displayName || receiverStatus.portName" class="usb-path">
            {{ receiverStatus.displayName || receiverStatus.portName }}
          </span>
        </div>
        <span class="tracker-cell-status">
          {{
            receiverStatus.inserted
              ? t("tracker_status.inserted")
              : t("tracker_status.not_inserted")
          }}
        </span>
      </div>

      <div 
        v-for="item in trackers" 
        :key="item.id" 
        class="tracker-cell" 
        :class="{ inserted: item.inserted }"
        @contextmenu="showContextMenu($event, item)"
      >
        <div class="tracker-info tracker-info--split">
          <div class="slot-identity-block">
            <span class="slot-name">{{ t('tracker_status.slot', { id: item.id }) }}</span>
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
          item.inserted ? t('tracker_status.inserted') : t('tracker_status.not_inserted')
        }}</span>
      </div>
    </div>

    <!-- 右键菜单 -->
    <Teleport to="body">
      <div 
        v-if="menuVisible" 
        class="context-menu" 
        :style="{ top: menuPosition.y + 'px', left: menuPosition.x + 'px' }"
        @click.stop
      >
        <div class="menu-header">{{ menuTitle }}</div>
        <div class="menu-item" @click="openConsole">
          {{ t("tracker_status.open_serial_console") }}
        </div>
        <template v-if="menuTarget?.kind === 'tracker'">
          <div 
            v-for="action in singleActions" 
            :key="action.value" 
            class="menu-item"
            :class="{ 'menu-item--disabled': !canRunTrackerActions }"
            @click="canRunTrackerActions && handleAction(action.value)"
          >
            {{ t(`tracker_control.${action.label}`) }}
          </div>
        </template>
      </div>
    </Teleport>
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

.receiver-cell {
  border-style: dashed;
}

.tracker-cell:hover {
  background: var(--color-secondary-hover);
}

.tracker-cell.inserted:hover {
  background: var(--color-success-hover);
}

/* 右键菜单样式 */
.context-menu {
  position: fixed;
  z-index: 1000;
  background: var(--color-bg-white);
  border: var(--border-width) solid var(--color-secondary);
  box-shadow: var(--box-shadow);
  min-width: 150px;
  padding: 4px 0;
}

.menu-header {
  padding: 6px 12px;
  font-weight: bold;
  border-bottom: var(--border-width-subtle) solid var(--color-secondary-hover);
  color: var(--color-text-secondary);
  font-size: 12px;
}

.menu-item {
  padding: 8px 12px;
  cursor: pointer;
  font-size: 14px;
  color: var(--color-text-main);
  transition: background 0.1s;
}

.menu-item:hover {
  background: var(--color-secondary-hover);
}

.menu-item--disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.menu-item--disabled:hover {
  background: transparent;
}
</style>
