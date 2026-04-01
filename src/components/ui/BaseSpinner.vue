<script setup lang="ts">
interface Props {
  size?: string;
  color?: string;
}

withDefaults(defineProps<Props>(), {
  size: '40px',
  color: 'var(--color-primary)',
});
</script>

<template>
  <div
    class="base-spinner"
    :style="{ width: size, height: size, borderColor: 'var(--color-spinner-track)', borderTopColor: color }"
  />
</template>

<style scoped>
.base-spinner {
  border: 4px solid;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}
</style>
