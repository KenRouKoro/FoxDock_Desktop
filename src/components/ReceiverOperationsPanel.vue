<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import BaseButton from "./ui/BaseButton.vue";
import BasePanel from "./ui/BasePanel.vue";
import BaseSelect from "./ui/BaseSelect.vue";
import ReceiverHidParamModal from "./ReceiverHidParamModal.vue";
import type {
  SerialConsoleLocalCommand,
  SerialConsoleRemoteCommand,
} from "../types/serialConsole";
import {
  buildReceiverContextMenuGroups,
  getReceiverRemoteCommands,
  remoteCommandUsesOnlyAll,
} from "../utils/serialConsoleCommands";

const { t } = useI18n();

const props = defineProps<{
  receiverInserted: boolean;
  loading?: boolean;
}>();

const emit = defineEmits<{
  (e: "runReceiverHidLine", line: string): void;
}>();

const panelDisabled = computed(() => props.loading || !props.receiverInserted);

const localGroups = computed(() => buildReceiverContextMenuGroups());
const remoteCommands = computed(() => getReceiverRemoteCommands());

const localParamOpen = ref(false);
const localParamCmd = ref<SerialConsoleLocalCommand | null>(null);

const remoteParamOpen = ref(false);
const remoteParamCmd = ref<SerialConsoleRemoteCommand | null>(null);

const remoteTargetChoice = ref<"all" | "id">("all");
const remoteTargetId = ref(0);

function clampRemoteId(raw: number): number {
  if (!Number.isFinite(raw)) return 0;
  return Math.min(255, Math.max(0, Math.floor(raw)));
}

function getRemoteTargetToken(): string {
  if (remoteTargetChoice.value === "all") return "all";
  return String(clampRemoteId(remoteTargetId.value));
}

function effectiveRemoteTargetFor(cmd: SerialConsoleRemoteCommand): string {
  return remoteCommandUsesOnlyAll(cmd) ? "all" : getRemoteTargetToken();
}

const effectiveRemoteForModal = computed(() =>
  remoteParamCmd.value ? effectiveRemoteTargetFor(remoteParamCmd.value) : "all",
);

function onLocalCommandClick(cmd: SerialConsoleLocalCommand): void {
  if (panelDisabled.value) return;
  if (cmd.parametric) {
    localParamCmd.value = cmd;
    localParamOpen.value = true;
    return;
  }
  if (cmd.command) {
    emit("runReceiverHidLine", cmd.command);
  }
}

function onRemoteCommandClick(cmd: SerialConsoleRemoteCommand): void {
  if (panelDisabled.value) return;
  if (cmd.parametric) {
    remoteParamCmd.value = cmd;
    remoteParamOpen.value = true;
    return;
  }
  const tail = cmd.remoteTail;
  if (!tail) return;
  const target = remoteCommandUsesOnlyAll(cmd) ? "all" : getRemoteTargetToken();
  emit("runReceiverHidLine", `send ${target} ${tail}`);
}

function onLocalParamSubmit(line: string): void {
  emit("runReceiverHidLine", line);
  localParamCmd.value = null;
}

function onRemoteParamSubmit(line: string): void {
  emit("runReceiverHidLine", line);
  remoteParamCmd.value = null;
}
</script>

<template>
  <div class="receiver-ops">
    <div class="receiver-ops__grid">
      <div class="receiver-ops__local">
        <section
          v-for="(group, idx) in localGroups"
          :key="group.labelKey"
          class="section-block"
          :class="{ 'section-block-last': idx === localGroups.length - 1 }"
        >
          <h3 class="section-title">{{ t(group.labelKey) }}</h3>
          <div class="button-grid">
            <BaseButton
              v-for="cmd in group.commands"
              :key="cmd.key"
              variant="outline"
              :disabled="panelDisabled"
              @click="onLocalCommandClick(cmd)"
            >
              {{ t(`serial_console.commands.${cmd.key}`) }}
            </BaseButton>
          </div>
        </section>
      </div>

      <BasePanel class="receiver-ops__remote-panel" :title="t('serial_console.remote_commands_title')">
        <section class="section-block section-block-last">
          <div class="remote-target-row">
            <span class="remote-target-label">{{ t("serial_console.remote_target_label") }}</span>
            <BaseSelect
              v-model="remoteTargetChoice"
              class="remote-target-mode"
              :disabled="panelDisabled"
            >
              <option value="all">{{ t("serial_console.remote_target_all") }}</option>
              <option value="id">{{ t("serial_console.remote_target_id") }}</option>
            </BaseSelect>
            <input
              v-show="remoteTargetChoice === 'id'"
              v-model.number="remoteTargetId"
              class="remote-target-id-input"
              type="number"
              min="0"
              max="255"
              step="1"
              :disabled="panelDisabled"
              :aria-label="t('serial_console.remote_target_id')"
            />
          </div>
          <p v-if="remoteTargetChoice === 'id'" class="remote-target-hint">
            {{ t("serial_console.remote_target_hint_id") }}
          </p>
          <div class="button-grid">
            <BaseButton
              v-for="cmd in remoteCommands"
              :key="cmd.key"
              variant="outline"
              :disabled="panelDisabled"
              @click="onRemoteCommandClick(cmd)"
            >
              {{ t(`serial_console.commands.${cmd.key}`) }}
            </BaseButton>
          </div>
        </section>
      </BasePanel>
    </div>

    <ReceiverHidParamModal
      v-model:open="localParamOpen"
      variant="local"
      :local-command="localParamCmd"
      @submit="onLocalParamSubmit"
    />

    <ReceiverHidParamModal
      v-model:open="remoteParamOpen"
      variant="remote"
      :remote-command="remoteParamCmd"
      :effective-remote-target="effectiveRemoteForModal"
      @submit="onRemoteParamSubmit"
    />
  </div>
</template>

<style scoped>
.receiver-ops__grid {
  display: grid;
  grid-template-columns: 1fr minmax(260px, 340px);
  gap: var(--spacing-md);
  align-items: start;
}

@media (max-width: 900px) {
  .receiver-ops__grid {
    grid-template-columns: 1fr;
  }
}

.receiver-ops__local {
  min-width: 0;
}

.receiver-ops__remote-panel {
  min-height: 0;
}

.section-block {
  margin-bottom: var(--spacing-md);
  padding-bottom: var(--spacing-md);
  border-bottom: var(--border-width) solid var(--color-secondary-hover);
}

.section-block-last {
  margin-bottom: 0;
  padding-bottom: 0;
  border-bottom: none;
}

.section-title {
  margin: 0 0 var(--spacing-sm);
  font-size: 14px;
  font-weight: 600;
  color: var(--color-text-secondary);
}

.button-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: var(--spacing-sm);
  margin-bottom: 0;
}

.remote-target-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--spacing-sm);
  margin-bottom: var(--spacing-xs);
}

.remote-target-label {
  font-size: 13px;
  color: var(--color-text-light);
  flex: 0 0 auto;
}

.remote-target-mode {
  flex: 1 1 120px;
  min-width: 100px;
  max-width: 160px;
}

.remote-target-id-input {
  width: 72px;
  min-height: 32px;
  padding: 6px 8px;
  border: var(--border-width) solid var(--color-border-control);
  border-radius: var(--border-radius);
  font: inherit;
  box-sizing: border-box;
}

.remote-target-hint {
  margin: 0 0 var(--spacing-sm);
  font-size: 12px;
  color: var(--color-text-light);
}
</style>
