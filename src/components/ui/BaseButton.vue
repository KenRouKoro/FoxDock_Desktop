<script setup lang="ts">
interface Props {
  variant?: 'primary' | 'secondary' | 'outline' | 'debug';
  disabled?: boolean;
}

withDefaults(defineProps<Props>(), {
  variant: 'primary',
  disabled: false,
});

const emit = defineEmits<{
  (e: 'click', event: MouseEvent): void;
}>();
</script>

<template>
  <button
    class="base-button"
    :class="[`variant-${variant}`]"
    :disabled="disabled"
    @click="emit('click', $event)"
  >
    <slot />
  </button>
</template>

<style scoped>
.base-button {
  border-width: var(--border-width);
  border-style: solid;
  border-radius: var(--border-radius);
  padding: 6px 12px;
  cursor: pointer;
  font-size: 14px;
  transition: all 0.2s;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-sm);
}

.variant-primary {
  background: var(--color-primary);
  color: var(--color-text-white);
  border-color: var(--color-primary);
}

.variant-primary:hover:not(:disabled) {
  background: var(--color-primary-hover);
}

.variant-secondary {
  background: var(--color-secondary);
  color: var(--color-text-white);
  border-color: var(--color-secondary);
}

.variant-outline {
  background: var(--color-bg-white);
  color: var(--color-primary);
  border-color: var(--color-primary);
}

.variant-outline:hover:not(:disabled) {
  background: var(--color-secondary-hover);
}

.variant-debug {
  background: var(--color-primary);
  color: var(--color-text-white);
  border-color: var(--color-primary);
  font-size: 12px;
}

.variant-debug:hover:not(:disabled) {
  background: var(--color-primary-hover);
}

.base-button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
</style>
