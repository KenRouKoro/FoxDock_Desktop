<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    progress: number;
    height?: string;
  }>(),
  {
    height: "12px",
  },
);

const pct = computed(() => Math.min(100, Math.max(0, props.progress)));
</script>

<template>
  <div
    class="base-progress-bar"
    :style="{ height }"
    role="progressbar"
    :aria-valuenow="Math.round(pct)"
    aria-valuemin="0"
    aria-valuemax="100"
  >
    <div class="base-progress-bar__fill" :style="{ width: `${pct}%` }" />
  </div>
</template>

<style scoped>
.base-progress-bar {
  width: 100%;
  background: var(--color-bg-page);
  border: var(--border-width) solid var(--color-primary);
  box-sizing: border-box;
}

.base-progress-bar__fill {
  height: 100%;
  background: var(--color-primary);
  transition: width 0.2s ease;
}
</style>
