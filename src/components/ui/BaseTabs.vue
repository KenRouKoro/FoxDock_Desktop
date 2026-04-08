<script setup lang="ts">
export interface BaseTabItem {
  key: string;
  label: string;
}

defineProps<{
  modelValue: string;
  tabs: BaseTabItem[];
  ariaLabel?: string;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", key: string): void;
}>();

function select(key: string): void {
  emit("update:modelValue", key);
}
</script>

<template>
  <nav class="base-tabs" role="tablist" :aria-label="ariaLabel">
    <button
      v-for="tab in tabs"
      :key="tab.key"
      type="button"
      role="tab"
      class="base-tabs__tab"
      :class="{ 'base-tabs__tab--active': modelValue === tab.key }"
      :aria-selected="modelValue === tab.key"
      @click="select(tab.key)"
    >
      {{ tab.label }}
    </button>
  </nav>
</template>

<style scoped>
.base-tabs {
  display: flex;
  border: var(--border-width) solid var(--color-secondary);
  background: var(--color-bg-header);
}

.base-tabs__tab {
  flex: 1;
  margin: 0;
  padding: var(--spacing-sm) var(--spacing-xs);
  border: none;
  border-right: var(--border-width-subtle) solid var(--color-secondary-hover);
  background: transparent;
  color: var(--color-text-light);
  font-size: 12px;
  font-weight: 700;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
}

.base-tabs__tab:last-child {
  border-right: none;
}

.base-tabs__tab:hover {
  background: var(--color-secondary-hover);
  color: var(--color-primary);
}

.base-tabs__tab--active {
  background: var(--color-bg-white);
  color: var(--color-primary);
  box-shadow: inset 0 -3px 0 var(--color-primary);
}
</style>
