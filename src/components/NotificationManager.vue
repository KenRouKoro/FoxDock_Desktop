<script setup lang="ts">
interface Notification {
  id: number;
  message: string;
  type: 'info' | 'success' | 'error';
  timestamp: number;
}

defineProps<{
  notifications: Notification[]
}>();
</script>

<template>
  <div class="notification-container">
    <TransitionGroup name="notification">
      <div 
        v-for="note in notifications" 
        :key="note.id" 
        class="notification-item" 
        :class="note.type"
      >
        <div class="notification-content">{{ note.message }}</div>
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.notification-container {
  position: fixed;
  top: calc(var(--spacing-xl) + 36px);
  right: var(--spacing-xl);
  z-index: 9999;
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
  pointer-events: none;
}

.notification-item {
  pointer-events: auto;
  min-width: 260px;
  max-width: 400px;
  padding: var(--spacing-md) var(--spacing-lg);
  background: var(--color-bg-white);
  border: var(--border-width) solid var(--color-secondary);
  box-shadow: var(--box-shadow);
  display: flex;
  align-items: center;
  font-size: 14px;
}

.notification-item.info {
  border-left: 6px solid var(--color-primary);
  background: #f3f9ff;
}

.notification-item.success {
  border-left: 6px solid var(--color-success);
  background: var(--color-success-bg);
}

.notification-item.error {
  border-left: 6px solid var(--color-error);
  background: var(--color-error-bg);
}

.notification-content {
  flex: 1;
  color: var(--color-text-main);
  word-break: break-all;
}

.notification-enter-active,
.notification-leave-active {
  transition: all 0.3s ease;
}

.notification-enter-from {
  opacity: 0;
  transform: translateX(30px);
}

.notification-leave-to {
  opacity: 0;
  transform: translateX(30px);
}
</style>
