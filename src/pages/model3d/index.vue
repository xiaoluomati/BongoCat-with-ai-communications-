<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { useModel3D } from '@/composables/useModel3D'
import { useModel3DStore } from '@/stores/model3d'

const store = useModel3DStore()
const { status, progress, error, init, loadCurrentModel, destroy } = useModel3D()
const appWindow = getCurrentWebviewWindow()

const canvasRef = ref<HTMLCanvasElement | null>(null)
let unlisten: UnlistenFn | null = null

onMounted(async () => {
  if (!canvasRef.value) return
  await init(canvasRef.value)

  // Load initial model
  await loadModelsAndShow()

  // Listen for cross-window updates
  unlisten = await listen('model3d-updated', async () => {
    await loadModelsAndShow()
  })
})

onUnmounted(() => {
  unlisten?.()
  destroy()
})

async function loadModelsAndShow() {
  // Reload from backend to get latest
  const { invoke } = await import('@tauri-apps/api/core')
  try {
    const list = await invoke<any[]>('list_3d_models')
    store.setModels(list)
    if (!store.currentModelId && list.length > 0) {
      store.selectModel(list[0].id)
    }
  } catch { /* ignore */ }
  if (store.currentPmxPath) {
    await loadCurrentModel()
  }
}
</script>

<template>
  <div class="model3d-container" @mousedown="appWindow.startDragging()">
    <div v-if="status === 'loading'" class="overlay">
      <div class="spinner" />
      <span>{{ progress }}%</span>
    </div>
    <div v-else-if="status === 'error'" class="overlay">
      <span class="error-text">{{ error }}</span>
    </div>
    <canvas ref="canvasRef" id="model3dCanvas" />
  </div>
</template>

<style scoped>
.model3d-container {
  width: 100%;
  height: 100vh;
  overflow: hidden;
  position: relative;
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
</style>
