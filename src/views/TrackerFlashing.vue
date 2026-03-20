<script setup lang="ts">
import { useI18n } from "vue-i18n";
import BasePanel from "../components/ui/BasePanel.vue";

const { t } = useI18n();
</script>

<template>
  <div class="flashing-page">
    <BasePanel :title="t('flashing.title')">
      <div class="placeholder">
        <p>{{ t('flashing.placeholder') }}</p>
      </div>
    </BasePanel>
  </div>
</template>

<style scoped>
.flashing-page {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
}

.placeholder {
  padding: var(--spacing-xl);
  text-align: center;
  color: var(--color-text-light);
  border: 1px dashed var(--color-secondary);
  background: var(--color-bg-panel);
}
</style>
