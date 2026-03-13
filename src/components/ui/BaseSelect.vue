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
  border: var(--border-width) solid var(--border-color);
  border-radius: var(--border-radius);
  background: var(--color-bg-white);
  color: var(--color-text-main);
  padding: 6px 10px;
  font-size: 14px;
  cursor: pointer;
  transition: border-color 0.2s;
  outline: none;
}

.base-select:focus {
  border-color: var(--color-primary);
}

.base-select:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
</style>
