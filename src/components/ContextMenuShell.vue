<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import type {
  ContextMenuParentPayload,
  ContextMenuSelection,
  ContextMenuSubmenuPayload,
} from "../types/contextMenu";
import {
  buildReceiverContextMenuGroups,
  findReceiverLocalCommandByKey,
} from "../utils/serialConsoleCommands";
import {
  clampWindowPosition,
  clientPointToScreenLogical,
  estimateReceiverSubmenuSize,
} from "../utils/contextMenu";

const { t } = useI18n();

const searchParams = new URLSearchParams(window.location.search);
const role = searchParams.get("role") as "parent" | "submenu" | null;

const parentPayload = ref<ContextMenuParentPayload | null>(null);
const subPayload = ref<ContextMenuSubmenuPayload | null>(null);
const parseError = ref("");
const loading = ref(true);

const receiverSubmenuGroups = computed(() => {
  if (!subPayload.value || subPayload.value.variant !== "receiver-commands") {
    return [];
  }
  return buildReceiverContextMenuGroups();
});

function applyPayloadFromBackend(raw: unknown): void {
  if (raw == null || typeof raw !== "object") {
    return;
  }
  const o = raw as Record<string, unknown>;
  if (role === "parent") {
    if (o.variant === "tracker" || o.variant === "receiver") {
      parentPayload.value = raw as ContextMenuParentPayload;
    }
  } else if (role === "submenu") {
    if (o.variant === "receiver-commands") {
      subPayload.value = raw as ContextMenuSubmenuPayload;
    }
  }
}

let unlistenState: (() => void) | undefined;

onMounted(async () => {
  window.addEventListener("keydown", onKeydown);
  if (!role || (role !== "parent" && role !== "submenu")) {
    parseError.value = "invalid role";
    loading.value = false;
    return;
  }

  unlistenState = await listen<unknown>("context-menu-state", (event) => {
    applyPayloadFromBackend(event.payload);
  });

  try {
    const initial = await invoke<unknown | null>("get_context_menu_state", { role });
    if (initial != null) {
      applyPayloadFromBackend(initial);
    } else {
      parseError.value = "empty menu state";
    }
  } catch (e) {
    parseError.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
});

onUnmounted(() => {
  unlistenState?.();
  window.removeEventListener("keydown", onKeydown);
});

const trackerParent = computed(() =>
  parentPayload.value?.variant === "tracker" ? parentPayload.value : null,
);
const receiverParent = computed(() =>
  parentPayload.value?.variant === "receiver" ? parentPayload.value : null,
);

const receiverCommandsRowRef = ref<HTMLElement | null>(null);

const singleActions = [
  { label: "actions.ret", value: "ret" },
  { label: "actions.bl", value: "bl" },
  { label: "actions.wake_up", value: "wake_up" },
  { label: "actions.sleep", value: "sleep" },
  { label: "actions.pair", value: "pair" },
];

function canRunTrackerActions(): boolean {
  const p = trackerParent.value;
  if (!p) return false;
  return p.inserted && !p.actionsDisabled;
}

async function emitSelection(sel: ContextMenuSelection) {
  await emit("context-menu-selection", sel);
}

async function onTrackerOpenConsole() {
  const p = trackerParent.value;
  if (!p) return;
  await emitSelection({ kind: "tracker-open-console", trackerId: p.trackerId });
}

async function onTrackerAction(action: string) {
  const p = trackerParent.value;
  if (!p || !canRunTrackerActions()) return;
  await emitSelection({ kind: "tracker-action", action, trackerId: p.trackerId });
}

async function onReceiverOpenConsole() {
  await emitSelection({ kind: "receiver-open-console" });
}

function onReceiverCommandsPointerDown(ev: PointerEvent) {
  if (ev.button !== 0) return;
  const p = receiverParent.value;
  if (!p?.canUseReceiverActions) return;
  void openReceiverSubmenu();
}

async function openReceiverSubmenu() {
  const p = receiverParent.value;
  if (!p) return;
  const row = receiverCommandsRowRef.value;
  if (!row) return;
  const groups = buildReceiverContextMenuGroups();
  let itemCount = 0;
  for (const g of groups) {
    itemCount += g.commands.length;
  }
  const { width: w, height: h } = estimateReceiverSubmenuSize(itemCount);
  const rect = row.getBoundingClientRect();
  const gap = 2;
  const topLeft = await clientPointToScreenLogical(rect.left, rect.top);
  const topRight = await clientPointToScreenLogical(rect.right, rect.top);
  let screenX = topRight.x + gap;
  let screenY = topLeft.y;
  const scr = window.screen as Screen & { availLeft?: number };
  const ax = scr.availLeft ?? 0;
  const aw = window.screen.availWidth;
  if (screenX + w > ax + aw - 8) {
    screenX = topLeft.x - w - gap;
  }
  const { x, y } = clampWindowPosition(screenX, screenY, w, h);
  const payload: ContextMenuSubmenuPayload = {
    variant: "receiver-commands",
    canUseReceiverActions: p.canUseReceiverActions,
  };
  await invoke("open_context_submenu_window", {
    screenX: x,
    screenY: y,
    width: w,
    height: h,
    payloadJson: JSON.stringify(payload),
  });
}

async function onSubmenuCommand(cmdKey: string) {
  if (!subPayload.value?.canUseReceiverActions) return;
  const cmd = findReceiverLocalCommandByKey(cmdKey);
  if (!cmd) return;
  if (cmd.parametric) {
    await emitSelection({ kind: "receiver-param", cmdKey });
  } else if (cmd.command) {
    await emitSelection({ kind: "receiver-hid-line", line: cmd.command });
  }
}

const menuTitle = computed(() => {
  const tr = trackerParent.value;
  if (tr) {
    return t("tracker_status.slot", { id: tr.trackerId });
  }
  if (receiverParent.value) {
    return t("tracker_status.receiver");
  }
  return "";
});

function onKeydown(ev: KeyboardEvent) {
  if (ev.key === "Escape") {
    ev.preventDefault();
    void invoke("close_context_menu_windows");
  }
}
</script>

<template>
  <div
    class="context-menu-root"
    :class="{ 'context-menu-root--submenu': role === 'submenu' }"
    @click.stop
  >
    <div v-if="loading" class="menu-loading">{{ t("common.loading") }}</div>

    <p v-else-if="parseError" class="menu-error">{{ parseError }}</p>

    <template v-else-if="trackerParent">
      <div class="menu-header">{{ menuTitle }}</div>
      <div class="menu-item" @click="onTrackerOpenConsole">
        {{ t("tracker_status.open_serial_console") }}
      </div>
      <div
        v-for="action in singleActions"
        :key="action.value"
        class="menu-item"
        :class="{ 'menu-item--disabled': !canRunTrackerActions() }"
        @click="canRunTrackerActions() && onTrackerAction(action.value)"
      >
        {{ t(`tracker_control.${action.label}`) }}
      </div>
    </template>

    <template v-else-if="receiverParent">
      <div class="menu-header">{{ menuTitle }}</div>
      <div
        class="menu-item"
        :class="{ 'menu-item--disabled': !receiverParent.canUseReceiverActions }"
        @click="receiverParent.canUseReceiverActions && onReceiverOpenConsole()"
      >
        {{ t("tracker_status.open_serial_console") }}
      </div>
      <div
        ref="receiverCommandsRowRef"
        class="menu-item menu-item--has-sub"
        @pointerdown.stop.prevent="onReceiverCommandsPointerDown"
      >
        <span>{{ t("tracker_status.receiver_commands") }}</span>
        <span class="submenu-arrow" aria-hidden="true">›</span>
      </div>
    </template>

    <template v-else-if="subPayload && subPayload.variant === 'receiver-commands'">
      <div class="submenu-scroll">
        <template v-for="group in receiverSubmenuGroups" :key="group.labelKey">
          <div class="submenu-group-label">{{ t(group.labelKey) }}</div>
          <div
            v-for="c in group.commands"
            :key="c.key"
            class="menu-item submenu-item"
            :class="{ 'menu-item--disabled': !subPayload.canUseReceiverActions }"
            @click="subPayload.canUseReceiverActions && onSubmenuCommand(c.key)"
          >
            {{ t(`serial_console.commands.${c.key}`) }}
          </div>
        </template>
      </div>
    </template>

    <p v-else-if="!loading" class="menu-error">{{ t("context_menu.invalid_state") }}</p>
  </div>
</template>

<style scoped>
.context-menu-root {
  width: 100%;
  height: 100%;
  min-height: 100%;
  margin: 0;
  box-sizing: border-box;
  overflow: hidden;
  user-select: none;
  background: var(--color-bg-white);
  color: var(--color-text-main);
  border: 2px solid var(--color-primary);
  box-shadow: none;
  outline: none;
}

.menu-loading {
  padding: 12px;
  font-size: 13px;
  color: var(--color-text-secondary);
}

.menu-error {
  padding: 8px 12px;
  font-size: 12px;
  color: var(--color-danger, #c00);
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
  position: relative;
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

.menu-item--has-sub {
  padding-right: 28px;
}

.submenu-arrow {
  position: absolute;
  right: 10px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 16px;
  color: var(--color-text-secondary);
}

.context-menu-root--submenu {
  display: flex;
  flex-direction: column;
}

.submenu-scroll {
  max-height: min(70vh, 480px);
  overflow-y: auto;
  padding: 4px 0;
}

.context-menu-root--submenu .submenu-scroll {
  flex: 1 1 auto;
  min-height: 0;
  max-height: none;
}

.submenu-group-label {
  padding: 6px 12px 4px;
  font-size: 11px;
  font-weight: bold;
  color: var(--color-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.02em;
}

.submenu-item {
  padding-left: 16px;
  font-size: 13px;
}

/* 独立菜单窗：避免 WebView 首帧露黑/灰底；100% 避免 100vw 导致底/右边框被裁切 */
:global(html),
:global(body) {
  margin: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: var(--color-bg-white);
}
</style>
