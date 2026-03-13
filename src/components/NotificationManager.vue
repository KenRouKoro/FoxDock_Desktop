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
  top: 20px;
  right: 20px;
  z-index: 9999;
  display: flex;
  flex-direction: column;
  gap: 10px;
  pointer-events: none;
}

.notification-item {
  pointer-events: auto;
  min-width: 260px;
  max-width: 400px;
  padding: 12px 16px;
  background: #ffffff;
  border: 2px solid #59a9ff;
  box-shadow: 4px 4px 0px rgba(0, 0, 0, 0.1);
  display: flex;
  align-items: center;
  font-size: 14px;
}

.notification-item.info {
  border-left: 6px solid #0078d4;
  background: #f3f9ff;
}

.notification-item.success {
  border-left: 6px solid #107c10;
  background: #f1faf1;
}

.notification-item.error {
  border-left: 6px solid #d83b01;
  background: #fff4f4;
}

.notification-content {
  flex: 1;
  color: #12304f;
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
