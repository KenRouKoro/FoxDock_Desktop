<script setup lang="ts">
import { ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();
const isAlwaysOnTop = ref(false);

const startDrag = async () => {
  try {
    await appWindow.startDragging();
  } catch (error) {
    console.error("[WindowTitleBar] startDragging failed:", error);
  }
};
const minimize = async () => {
  try {
    await appWindow.minimize();
  } catch (error) {
    console.error("[WindowTitleBar] minimize failed:", error);
  }
};
const close = async () => {
  try {
    await appWindow.close();
  } catch (error) {
    console.error("[WindowTitleBar] close failed:", error);
  }
};
const toggleAlwaysOnTop = async () => {
  const next = !isAlwaysOnTop.value;
  try {
    await appWindow.setAlwaysOnTop(next);
    isAlwaysOnTop.value = next;
  } catch (error) {
    console.error("[WindowTitleBar] setAlwaysOnTop failed:", error);
  }
};
</script>

<template>
  <div class="titlebar" @mousedown.left="startDrag">
    <div class="drag-region"></div>
    <div class="titlebar-actions">
      <div
        class="titlebar-button"
        :class="{ active: isAlwaysOnTop }"
        @mousedown.stop
        @click="toggleAlwaysOnTop"
        :title="isAlwaysOnTop ? '取消置顶' : '窗口置顶'"
      >
        <svg v-if="isAlwaysOnTop" width="14" height="14"  viewBox="0 0 1024 1024"  fill="currentColor" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M110.08 913.984a48 48 0 0 1 0-67.904l270.848-271.04-195.264-195.136a48.256 48.256 0 0 1 0-67.968 371.2 371.2 0 0 1 264.192-109.184 344.384 344.384 0 0 1 78.784 9.472l102.4-102.4a48 48 0 0 1 67.904 0l215.04 215.04a48 48 0 0 1 0 67.84L812.8 493.888a344.64 344.64 0 0 1 10.496 81.92 371.904 371.904 0 0 1-109.376 264 48 48 0 0 1-67.84 0l-197.184-197.12-270.976 270.976a48.064 48.064 0 0 1-67.84 0z m180.288-565.248l386.688 386.624a268.416 268.416 0 0 0 36.224-241.216 47.872 47.872 0 0 1 11.904-48.128l86.912-86.976-147.2-147.2-87.936 87.936a48 48 0 0 1-47.744 12.096 267.2 267.2 0 0 0-79.36-13.312 276.608 276.608 0 0 0-159.488 50.176z" fill="#ffffff" p-id="1713"></path>
        </svg>
        <svg v-else width="14" height="14"  viewBox="0 0 1024 1024"  fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M110.08 913.984a48 48 0 0 1 0-67.904l270.848-271.04-195.264-195.136a48.256 48.256 0 0 1 0-67.968 371.2 371.2 0 0 1 264.192-109.184 344.384 344.384 0 0 1 78.784 9.472l102.4-102.4a48 48 0 0 1 67.904 0l215.04 215.04a48 48 0 0 1 0 67.84L812.8 493.888a344.64 344.64 0 0 1 10.496 81.92 371.904 371.904 0 0 1-109.376 264 48 48 0 0 1-67.84 0l-197.184-197.12-270.976 270.976a48.064 48.064 0 0 1-67.84 0z m180.288-565.248l386.688 386.624a268.416 268.416 0 0 0 36.224-241.216 47.872 47.872 0 0 1 11.904-48.128l86.912-86.976-147.2-147.2-87.936 87.936a48 48 0 0 1-47.744 12.096 267.2 267.2 0 0 0-79.36-13.312 276.608 276.608 0 0 0-159.488 50.176z" fill="#ffffff" p-id="1713"></path>
        </svg>
      </div>
      <div class="titlebar-button" @mousedown.stop @click="minimize">
        <svg width="10" height="1" viewBox="0 0 10 1" fill="none" xmlns="http://www.w3.org/2000/svg">
          <rect width="10" height="1" fill="currentColor"/>
        </svg>
      </div>
      <div class="titlebar-button close" @mousedown.stop @click="close">
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M1 1L9 9M1 9L9 1" stroke="currentColor" stroke-width="1.5"/>
        </svg>
      </div>
    </div>
  </div>
</template>

<style scoped>
.titlebar {
  height: 32px;
  background: var(--color-primary);
  color: var(--color-text-white);
  display: flex;
  justify-content: space-between;
  align-items: center;
  user-select: none;
  position: relative;
  z-index: 10000;
}

.drag-region {
  flex: 1;
  height: 100%;
}

.titlebar-actions {
  display: flex;
  height: 100%;
}

.titlebar-button {
  width: 44px;
  height: 100%;
  display: flex;
  justify-content: center;
  align-items: center;
  cursor: pointer;
  transition: background 0.2s;
}

.titlebar-button:hover {
  background: var(--color-primary-hover);
}

.titlebar-button.active {
  background: var(--color-secondary);
  color: var(--color-text-white);
}

.titlebar-button.active:hover {
  background: var(--color-secondary-hover);
  color: var(--color-primary);
}

.titlebar-button svg {
  transition: transform 0.1s;
}

.titlebar-button:active svg {
  transform: scale(0.9);
}

.titlebar-button.close:hover {
  background: #e81123;
}
</style>
