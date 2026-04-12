<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import BaseButton from "./ui/BaseButton.vue";
import BaseSelect from "./ui/BaseSelect.vue";
import type {
  SerialConsoleField,
  SerialConsoleLocalCommand,
  SerialConsoleRemoteCommand,
} from "../types/serialConsole";
import {
  initParamFormValues,
  tryBuildLocalLine,
  tryBuildRemoteTail,
} from "../utils/serialConsoleCommands";

const { t } = useI18n();

const props = defineProps<{
  open: boolean;
  variant: "local" | "remote";
  localCommand?: SerialConsoleLocalCommand | null;
  remoteCommand?: SerialConsoleRemoteCommand | null;
  /** 远程：已解析的 send 目标（含 remoteOnlyAll 时为 all）；本地时可省略 */
  effectiveRemoteTarget?: string;
}>();

const emit = defineEmits<{
  (e: "update:open", value: boolean): void;
  (e: "submit", line: string): void;
}>();

const paramFormValues = ref<Record<string, string | number | boolean>>({});
const paramModalError = ref("");

function resetForm(): void {
  paramModalError.value = "";
  if (props.variant === "local" && props.localCommand?.parametric) {
    paramFormValues.value = initParamFormValues(props.localCommand.parametric.fields);
  } else if (props.variant === "remote" && props.remoteCommand?.parametric) {
    paramFormValues.value = initParamFormValues(props.remoteCommand.parametric.fields);
  } else {
    paramFormValues.value = {};
  }
}

watch(
  () => props.open,
  (v) => {
    if (v) resetForm();
  },
);

watch(
  () =>
    props.variant === "local"
      ? props.localCommand?.key
      : props.remoteCommand?.key,
  () => {
    if (props.open) resetForm();
  },
);

const paramModalFields = computed((): SerialConsoleField[] => {
  if (!props.open) return [];
  if (props.variant === "local") {
    return props.localCommand?.parametric?.fields ?? [];
  }
  return props.remoteCommand?.parametric?.fields ?? [];
});

const paramPreview = computed(() => {
  if (!props.open) return { ok: true as const, text: "" };
  if (props.variant === "local" && props.localCommand?.parametric) {
    return tryBuildLocalLine(props.localCommand, paramFormValues.value);
  }
  if (props.variant === "remote" && props.remoteCommand?.parametric) {
    const tail = tryBuildRemoteTail(props.remoteCommand, paramFormValues.value);
    if (!tail.ok) return tail;
    const tgt = props.effectiveRemoteTarget ?? "all";
    return { ok: true as const, text: `send ${tgt} ${tail.text}` };
  }
  return { ok: true as const, text: "" };
});

function close(): void {
  emit("update:open", false);
  paramModalError.value = "";
}

function submit(): void {
  const preview = paramPreview.value;
  if (!preview.ok) {
    paramModalError.value = preview.messageParams
      ? t(preview.messageKey, preview.messageParams)
      : t(preview.messageKey);
    return;
  }
  paramModalError.value = "";
  emit("submit", preview.text);
  close();
}

function onParamBackdropClick(event: MouseEvent): void {
  if ((event.target as HTMLElement).classList.contains("param-modal-backdrop")) {
    close();
  }
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape" && props.open) {
    event.preventDefault();
    close();
  }
}

const subHint = computed(() =>
  props.variant === "local"
    ? t("tracker_status.receiver_hid_param_hint")
    : t("serial_console.param_modal_remote_note"),
);

onMounted(() => {
  window.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="param-modal-backdrop"
      role="presentation"
      @click="onParamBackdropClick"
    >
      <div
        class="param-modal"
        role="dialog"
        aria-modal="true"
        :aria-label="t('serial_console.param_modal_title')"
        @click.stop
      >
        <h3 class="param-modal-title">{{ t("serial_console.param_modal_title") }}</h3>
        <p class="param-modal-sub">{{ subHint }}</p>
        <form class="param-modal-form" @submit.prevent="submit">
          <div v-for="field in paramModalFields" :key="field.id" class="param-field">
            <label
              v-if="field.kind !== 'toggle'"
              class="param-field-label"
              :for="`rhid-param-${field.id}`"
              >{{ t(field.labelKey) }}</label
            >
            <template v-if="field.kind === 'text'">
              <input
                :id="`rhid-param-${field.id}`"
                v-model="paramFormValues[field.id] as string"
                class="param-field-control"
                type="text"
                :placeholder="
                  field.placeholderKey ? t(field.placeholderKey) : undefined
                "
                autocomplete="off"
              />
            </template>
            <template v-else-if="field.kind === 'number'">
              <input
                :id="`rhid-param-${field.id}`"
                v-model="paramFormValues[field.id]"
                class="param-field-control"
                type="number"
                :min="field.min"
                :max="field.max"
                :step="field.integer ? 1 : 'any'"
              />
            </template>
            <template v-else-if="field.kind === 'select'">
              <BaseSelect
                :id="`rhid-param-${field.id}`"
                v-model="paramFormValues[field.id] as string"
                class="param-field-control param-field-select"
              >
                <option
                  v-for="opt in field.options"
                  :key="opt.value"
                  :value="opt.value"
                >
                  {{ t(opt.labelKey) }}
                </option>
              </BaseSelect>
            </template>
            <template v-else-if="field.kind === 'toggle'">
              <label class="param-toggle-row" :for="`rhid-param-${field.id}`">
                <input
                  :id="`rhid-param-${field.id}`"
                  v-model="paramFormValues[field.id] as boolean"
                  type="checkbox"
                />
                <span>{{ t(field.labelKey) }}</span>
              </label>
            </template>
          </div>
          <div class="param-preview">
            <span class="param-preview-label">{{ t("serial_console.param_preview") }}</span>
            <code class="param-preview-code">{{
              paramPreview.ok ? paramPreview.text : "—"
            }}</code>
            <p v-if="!paramPreview.ok" class="param-preview-error">
              {{
                paramPreview.messageParams
                  ? t(paramPreview.messageKey, paramPreview.messageParams)
                  : t(paramPreview.messageKey)
              }}
            </p>
          </div>
          <p v-if="paramModalError" class="param-modal-error">{{ paramModalError }}</p>
          <div class="param-modal-actions">
            <BaseButton type="button" variant="outline" @click="close">
              {{ t("serial_console.param_modal_cancel") }}
            </BaseButton>
            <BaseButton type="submit">
              {{ t("tracker_status.receiver_hid_send") }}
            </BaseButton>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.param-modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 9998;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-md);
}

.param-modal {
  width: min(420px, 100%);
  background: var(--color-bg-white);
  border: var(--border-width) solid var(--color-secondary);
  box-shadow: var(--box-shadow-heavy);
  padding: var(--spacing-md);
  max-height: min(90vh, 520px);
  overflow: auto;
}

.param-modal-title {
  margin: 0 0 var(--spacing-xs);
  font-size: 16px;
}

.param-modal-sub {
  margin: 0 0 var(--spacing-md);
  font-size: 12px;
  color: var(--color-text-secondary);
}

.param-modal-form {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
}

.param-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.param-field-label {
  font-size: 12px;
  color: var(--color-text-secondary);
}

.param-field-control {
  padding: 6px 8px;
  border: var(--border-width) solid var(--color-secondary);
  border-radius: var(--border-radius-sm);
  font-size: 14px;
}

.param-field-select {
  width: 100%;
}

.param-toggle-row {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  font-size: 14px;
}

.param-preview {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
}

.param-preview-label {
  color: var(--color-text-secondary);
}

.param-preview-code {
  font-family: var(--font-family-mono);
  font-size: 12px;
  color: var(--color-text-main);
  word-break: break-all;
}

.param-preview-error {
  color: var(--color-danger, #c00);
  margin: 0;
}

.param-modal-error {
  color: var(--color-danger, #c00);
  font-size: 12px;
  margin: 0;
}

.param-modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-sm);
  margin-top: var(--spacing-sm);
}
</style>
