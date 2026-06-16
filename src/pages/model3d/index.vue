<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { useModel3D, type Status } from '@/composables/useModel3D'
import { useModel3DStore } from '@/stores/model3d'

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
</script>

<template>
  <div class="model3d-container">
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

    <!-- Canvas -->
    <canvas ref="canvasRef" id="model3dCanvas" />
  </div>
</template>

<style scoped>
.model3d-container {
  position: relative;
  width: 100%;
  height: 100vh;
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
