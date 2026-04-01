<script setup lang="ts">
import { computed, useAttrs } from "vue";

defineOptions({ inheritAttrs: false });

interface Props {
  modelValue: string | number;
  disabled?: boolean;
}

withDefaults(defineProps<Props>(), {
  disabled: false,
});

const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
}>();

const attrs = useAttrs();

/** 布局类放在包裹层，其余属性（含 id、data-*）交给 select */
const wrapperClass = computed(() => attrs.class);

const selectAttrs = computed(() => {
  const { class: _c, ...rest } = attrs as Record<string, unknown>;
  return rest;
});

const handleChange = (event: Event) => {
  const target = event.target as HTMLSelectElement;
  emit("update:modelValue", target.value);
};
</script>

<template>
  <div class="base-select-wrap" :class="wrapperClass">
    <select
      class="base-select"
      :value="modelValue"
      :disabled="disabled"
      v-bind="selectAttrs"
      @change="handleChange"
    >
      <slot />
    </select>
  </div>
</template>

<style scoped>
.base-select-wrap {
  position: relative;
  display: inline-block;
  min-width: 0;
  vertical-align: middle;
}

.base-select-wrap::after {
  content: "";
  position: absolute;
  right: 10px;
  top: 50%;
  transform: translateY(-50%);
  width: 0;
  height: 0;
  border-left: 5px solid transparent;
  border-right: 5px solid transparent;
  border-top: 6px solid var(--color-primary);
  pointer-events: none;
}

.base-select {
  appearance: none;
  -webkit-appearance: none;
  -moz-appearance: none;
  border: var(--border-width) solid var(--color-border-control);
  border-radius: var(--border-radius);
  background-color: var(--color-bg-white);
  color: var(--color-text-main);
  padding: 6px 32px 6px 12px;
  min-height: 32px;
  font-size: 14px;
  line-height: 1.25;
  cursor: pointer;
  transition: background-color 0.2s, border-color 0.2s, box-shadow 0.2s;
  outline: none;
  width: 100%;
}

.base-select:hover:not(:disabled) {
  background-color: var(--color-bg-control-hover);
}

.base-select:focus:not(:disabled),
.base-select:focus-visible:not(:disabled) {
  border-color: var(--color-primary);
  box-shadow: inset 0 0 0 var(--border-width-subtle) var(--color-primary);
}

.base-select:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
</style>
