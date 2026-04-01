<script setup lang="ts">
interface Props {
  modelValue: string | number;
  disabled?: boolean;
}

withDefaults(defineProps<Props>(), {
  disabled: false,
});

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void;
}>();

const handleInput = (event: Event) => {
  const target = event.target as HTMLSelectElement;
  emit('update:modelValue', target.value);
};
</script>

<template>
  <select
    class="base-select"
    :value="modelValue"
    :disabled="disabled"
    @input="handleInput"
  >
    <slot />
  </select>
</template>

<style scoped>
.base-select {
  appearance: none;
  -webkit-appearance: none;
  -moz-appearance: none;
  border: var(--border-width) solid var(--color-border-control);
  border-radius: var(--border-radius);
  background-color: var(--color-bg-white);
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'%3E%3Cpath fill='%230078d4' d='M7 10l5 5 5-5z'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 8px center;
  background-size: 18px 18px;
  color: var(--color-text-main);
  padding: 6px 32px 6px 12px;
  min-height: 32px;
  font-size: 14px;
  line-height: 1.25;
  cursor: pointer;
  transition: background-color 0.2s, border-color 0.2s, box-shadow 0.2s;
  outline: none;
}

.base-select:hover:not(:disabled) {
  background-color: var(--color-bg-control-hover);
}

.base-select:focus:not(:disabled),
.base-select:focus-visible:not(:disabled) {
  border-color: var(--color-primary);
  box-shadow: inset 0 0 0 1px var(--color-primary);
}

.base-select:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
</style>
