<script setup lang="ts">
interface Props {
  title?: string;
  padding?: string;
}

withDefaults(defineProps<Props>(), {
  padding: 'var(--spacing-md)',
});
</script>

<template>
  <section class="base-panel">
    <header v-if="title || $slots.header" class="panel-header">
      <slot name="header">
        <h2 class="panel-title">{{ title }}</h2>
      </slot>
    </header>
    <div class="panel-content" :style="{ padding }">
      <slot />
    </div>
    <footer v-if="$slots.footer" class="panel-footer">
      <slot name="footer" />
    </footer>
  </section>
</template>

<style scoped>
.base-panel {
  border: var(--border-width) solid var(--border-color);
  background: var(--color-bg-panel);
  margin-bottom: var(--spacing-md);
  display: flex;
  flex-direction: column;
}

.panel-header {
  padding: var(--spacing-sm) var(--spacing-md);
  border-bottom: 1px solid var(--color-secondary-hover);
  background: var(--color-bg-header);
}

.panel-title {
  margin: 0;
  font-size: 18px;
  color: var(--color-text-secondary);
}

.panel-content {
  flex: 1;
}

.panel-footer {
  padding: var(--spacing-sm) var(--spacing-md);
  border-top: 1px solid var(--color-secondary-hover);
}
</style>
