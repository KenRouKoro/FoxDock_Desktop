<script setup lang="ts">
defineProps<{
  title: string;
  description?: string;
  logoSrc: string;
  logoAlt?: string;
}>();
</script>

<template>
  <header class="page-header">
    <div class="page-header__bar">
      <div class="page-header__title-area">
        <img :src="logoSrc" class="page-header__logo" :alt="logoAlt ?? 'Logo'" />
        <div class="page-header__text">
          <h1>{{ title }}</h1>
          <p v-if="description">{{ description }}</p>
        </div>
      </div>
    </div>
  </header>
</template>

<style scoped>
.page-header {
  border: var(--border-width) solid var(--color-secondary);
  background: var(--color-bg-header);
  padding: var(--spacing-xs) var(--spacing-md);
}

.page-header__bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.page-header__title-area {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
}

.page-header__logo {
  height: 28px;
  width: auto;
}

.page-header__text h1 {
  margin: 0;
  font-size: 15px;
  line-height: 1.2;
}

.page-header__text p {
  margin: 0;
  font-size: 11px;
  color: var(--color-text-light);
}
</style>
