<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { useModel3DStore, type Model3DInfo } from '@/stores/model3d'

const store = useModel3DStore()
const appWindow = getCurrentWebviewWindow()
const canvasRef = ref<HTMLCanvasElement | null>(null)
const status = ref<'init' | 'loading' | 'ready' | 'error'>('init')
const progress = ref(0)
const errorMsg = ref('')
const debugInfo = ref<string[]>([])
let unlisten: UnlistenFn | null = null

function log(msg: string) {
  console.log('[model3d]', msg)
  debugInfo.value.push(msg)
}

onMounted(async () => {
  log('page mounted')
  await nextTick()

  const canvas = canvasRef.value
  if (!canvas) {
    log('ERROR: canvas ref is null')
    return
  }
  log(`canvas size: ${canvas.clientWidth}x${canvas.clientHeight}`)

  // Dynamic import to catch module load errors
  try {
    const { useModel3D } = await import('@/composables/useModel3D')
    const model3d = useModel3D()

    log('initializing three.js scene...')
    await model3d.init(canvas)
    log('scene initialized')

    // Initial load
    await refreshAndLoad(model3d)

    // Listen for config changes
    unlisten = await listen('model3d-updated', async () => {
      log('event received: model3d-updated')
      await refreshAndLoad(model3d)
    })
  } catch (e: any) {
    log(`FATAL: ${e.message || String(e)}`)
    status.value = 'error'
    errorMsg.value = e.message || String(e)
  }
})

onUnmounted(() => {
  unlisten?.()
})

async function refreshAndLoad(model3d: any) {
  try {
    status.value = 'loading'
    const list = await invoke<Model3DInfo[]>('list_3d_models')
    log(`backend returned ${list.length} models`)
    if (list.length > 0) {
      log(`first model: id=${list[0].id}, name=${list[0].name}, pmx_path=${list[0].pmx_path}`)
    }

    if (list.length === 0) {
      status.value = 'init'
      return
    }

    store.setModels(list)
    // Force select the first model
    store.selectModel(list[0].id)
    log(`after select: currentModelId=${store.currentModelId}, pmxPath=${store.currentPmxPath}`)

    const pmxPath = store.currentPmxPath
    if (!pmxPath) {
      log('ERROR: currentPmxPath is null despite having models!')
      status.value = 'init'
      return
    }

    progress.value = 0
    await model3d.loadCurrentModel()
    log('model loaded successfully')
    status.value = 'ready'
  } catch (e: any) {
    log(`load error: ${e.message || String(e)}`)
    status.value = 'error'
    errorMsg.value = e.message || String(e)
  }
}
</script>

<template>
  <div class="model3d-container" @mousedown="appWindow.startDragging()">
    <div v-if="status === 'init'" class="overlay">
      <div class="debug-text">
        <div v-for="(line, i) in debugInfo" :key="i">{{ line }}</div>
        <div v-if="debugInfo.length === 0">等待加载...</div>
      </div>
    </div>
    <div v-else-if="status === 'loading'" class="overlay">
      <div class="spinner" />
      <span>{{ progress }}%</span>
    </div>
    <div v-else-if="status === 'error'" class="overlay">
      <span class="error-text">{{ errorMsg }}</span>
    </div>
    <canvas ref="canvasRef" id="model3dCanvas" />
  </div>
</template>

<style scoped>
.model3d-container {
  width: 100%; height: 100vh; overflow: hidden; position: relative;
  background: rgba(0,0,0,0.05);
}
#model3dCanvas { width: 100%; height: 100%; display: block; }
.overlay {
  position: absolute; inset: 0; display: flex;
  flex-direction: column; align-items: center; justify-content: center;
  gap: 12px; color: #888; font-size: 14px; z-index: 10; pointer-events: none;
}
.debug-text {
  text-align: left; font-size: 11px; font-family: monospace;
  max-width: 90%; word-break: break-all; line-height: 1.6; color: #666;
}
.spinner {
  width: 24px; height: 24px; border: 3px solid rgba(0,0,0,0.1);
  border-top-color: #ff9a7a; border-radius: 50%; animation: spin 0.8s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }
.error-text { color: #e44; max-width: 80%; text-align: center; }
</style>
