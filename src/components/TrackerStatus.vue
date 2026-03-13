<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import BasePanel from "./ui/BasePanel.vue";

const { t } = useI18n();

type TrackerStatus = {
  id: number;
  inserted: boolean;
  usbPath?: string;
};

const props = defineProps<{
  trackers: TrackerStatus[];
  disabled?: boolean;
}>();

const emit = defineEmits<{
  (e: 'runSingleAction', action: string, id: number): void;
}>();

// --- 右键菜单逻辑 ---
const menuVisible = ref(false);
const menuPosition = ref({ x: 0, y: 0 });
const activeTrackerId = ref<number | null>(null);

const showContextMenu = (event: MouseEvent, item: TrackerStatus) => {
  if (props.disabled || !item.inserted) return;
  event.preventDefault();
  activeTrackerId.value = item.id;
  menuPosition.value = { x: event.clientX, y: event.clientY };
  menuVisible.value = true;
};

const closeMenu = () => {
  menuVisible.value = false;
};

const handleAction = (action: string) => {
  if (activeTrackerId.value !== null) {
    emit('runSingleAction', action, activeTrackerId.value);
  }
  closeMenu();
};

const singleActions = [
  { label: "actions.ret", value: "ret" },
  { label: "actions.sleep", value: "sleep" },
  { label: "actions.bl", value: "bl" },
  { label: "actions.pair", value: "pair" },
];

onMounted(() => {
  window.addEventListener('click', closeMenu);
});

onUnmounted(() => {
  window.removeEventListener('click', closeMenu);
});
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
        <div class="tracker-info">
          <span class="slot-name">{{ t('tracker_status.slot', { id: item.id }) }}</span>
          <span v-if="item.usbPath" class="usb-path">{{ item.usbPath }}</span>
        </div>
        <span>{{ item.inserted ? t('tracker_status.inserted') : t('tracker_status.not_inserted') }}</span>
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
        <div class="menu-header">{{ t('tracker_status.slot', { id: activeTrackerId }) }}</div>
        <div 
          v-for="action in singleActions" 
          :key="action.value" 
          class="menu-item"
          @click="handleAction(action.value)"
        >
          {{ t(`tracker_control.${action.label}`) }}
        </div>
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
  background: #eef7ff;
  padding: var(--spacing-sm);
  display: flex;
  justify-content: space-between;
  align-items: center;
  cursor: default;
  user-select: none;
}

.tracker-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.slot-name {
  font-weight: bold;
}

.usb-path {
  font-size: 11px;
  color: var(--color-secondary);
  font-family: var(--font-family-mono);
}

.tracker-cell.inserted .usb-path {
  color: var(--color-success-border);
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
  border-bottom: 1px solid var(--color-secondary-hover);
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
</style>
