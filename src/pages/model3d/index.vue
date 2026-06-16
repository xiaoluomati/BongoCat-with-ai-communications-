<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { useModel3D, type Status } from '@/composables/useModel3D'
import { useModel3DStore } from '@/stores/model3d'

const appWindow = getCurrentWebviewWindow()
const store = useModel3DStore()
const { status, progress, error, init, loadCurrentModel, destroy } = useModel3D()

const canvasRef = ref<HTMLCanvasElement | null>(null)
const isReady = ref(false)

onMounted(async () => {
  if (!canvasRef.value) return
  await init(canvasRef.value)
  isReady.value = true
  if (store.currentPmxPath) {
    await loadCurrentModel()
  }
})

watch(() => store.currentPmxPath, async (path) => {
  if (path && isReady.value) {
    await loadCurrentModel()
  }
})

onUnmounted(() => destroy())

async function handleClose() {
  await appWindow.hide()
}
</script>

<template>
  <div class="model3d-container">
    <!-- Header -->
    <div class="model3d-header" data-tauri-drag-region>
      <span class="header-title">3D 模型</span>
      <button class="header-close" @click="handleClose">&times;</button>
    </div>

    <!-- Canvas area -->
    <div class="canvas-area">
      <!-- Loading -->
      <div v-if="status === 'loading'" class="overlay">
        <div class="spinner" />
        <span>加载中... {{ progress }}%</span>
      </div>

      <!-- Error -->
      <div v-else-if="status === 'error'" class="overlay">
        <span class="error-text">加载失败: {{ error }}</span>
      </div>

      <!-- Empty -->
      <div v-else-if="status === 'empty'" class="overlay">
        <span class="empty-text">请在设置中导入 PMX 模型</span>
      </div>

      <canvas ref="canvasRef" id="model3dCanvas" />
    </div>
  </div>
</template>

<style scoped>
.model3d-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: transparent;
}

.model3d-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  background: rgba(0, 0, 0, 0.3);
  flex-shrink: 0;
  height: 32px;
}

.header-title {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.6);
  user-select: none;
}

.header-close {
  background: none;
  border: none;
  color: rgba(255, 255, 255, 0.6);
  font-size: 18px;
  cursor: pointer;
  padding: 0 4px;
  line-height: 1;
  -webkit-app-region: no-drag;
}

.header-close:hover {
  color: #fff;
}

.canvas-area {
  flex: 1;
  position: relative;
  overflow: hidden;
}

#model3dCanvas {
  width: 100%;
  height: 100%;
}

.overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: #888;
  font-size: 14px;
  z-index: 10;
  pointer-events: none;
}

.spinner {
  width: 24px;
  height: 24px;
  border: 3px solid rgba(255, 255, 255, 0.2);
  border-top-color: #ff9a7a;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.error-text { color: #e88; }
.empty-text { color: #aaa; }
</style>
