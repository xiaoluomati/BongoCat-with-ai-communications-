<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { useModel3D } from '@/composables/useModel3D'
import { useModel3DStore, type Model3DInfo } from '@/stores/model3d'

const store = useModel3DStore()
const { status, progress, error, init, loadCurrentModel, destroy } = useModel3D()
const appWindow = getCurrentWebviewWindow()
const canvasRef = ref<HTMLCanvasElement | null>(null)
let unlisten: UnlistenFn | null = null

onMounted(async () => {
  if (!canvasRef.value) return
  await init(canvasRef.value)

  // Initial load from backend
  await refreshAndLoad()

  // Listen for config window changes
  unlisten = await listen('model3d-updated', async () => {
    console.log('[model3d] received model3d-updated event')
    await refreshAndLoad()
  })
})

onUnmounted(() => {
  unlisten?.()
  destroy()
})

async function refreshAndLoad() {
  try {
    const list = await invoke<Model3DInfo[]>('list_3d_models')
    console.log('[model3d] loaded models:', list.length)
    store.setModels(list)

    // Auto-select first if nothing selected
    if (!store.currentModelId && list.length > 0) {
      store.selectModel(list[0].id)
    }

    if (store.currentPmxPath) {
      console.log('[model3d] loading model:', store.currentPmxPath)
      await loadCurrentModel()
    } else {
      console.log('[model3d] no model selected')
    }
  } catch (e) {
    console.error('[model3d] refresh failed:', e)
  }
}
</script>

<template>
  <div class="model3d-container" @mousedown="appWindow.startDragging()">
    <div v-if="status === 'loading'" class="overlay">
      <div class="spinner" />
      <span>加载中... {{ progress }}%</span>
    </div>
    <div v-else-if="status === 'error'" class="overlay">
      <span class="error-text">加载失败: {{ error }}</span>
    </div>
    <div v-else-if="status === 'empty'" class="overlay">
      <span class="empty-text">请在综合功能中导入 PMX 模型</span>
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

#model3dCanvas { width: 100%; height: 100%; }

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
  width: 24px; height: 24px;
  border: 3px solid rgba(255,255,255,0.2);
  border-top-color: #ff9a7a;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin { to { transform: rotate(360deg); } }
.error-text { color: #e88; }
.empty-text { color: #aaa; }
</style>
