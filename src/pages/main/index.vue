<script setup lang="ts">
import { convertFileSrc } from '@tauri-apps/api/core'
import { PhysicalSize } from '@tauri-apps/api/dpi'
import { Menu } from '@tauri-apps/api/menu'
import { sep } from '@tauri-apps/api/path'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { exists, readDir } from '@tauri-apps/plugin-fs'
import { useDebounceFn, useEventListener } from '@vueuse/core'
import { round } from 'es-toolkit'
import { nth } from 'es-toolkit/compat'
import { onMounted, onUnmounted, ref, watch } from 'vue'

import { useDevice } from '@/composables/useDevice'
import { useGamepad } from '@/composables/useGamepad'
import { useModel } from '@/composables/useModel'
import { useSharedMenu } from '@/composables/useSharedMenu'
import { hideWindow, setAlwaysOnTop, setTaskbarVisibility, showWindow } from '@/plugins/window'
import { useCatStore } from '@/stores/cat'
import { useGeneralStore } from '@/stores/general.ts'
import { useModelStore } from '@/stores/model'
import { useModel3DStore } from '@/stores/model3d'
import { isImage } from '@/utils/is'
import { join } from '@/utils/path'
import { clearObject } from '@/utils/shared'
import { Live2DEngine } from '@/engines/Live2DEngine'
import { ThreeEngine } from '@/engines/ThreeEngine'
import type { DisplayEngine, ModelSize } from '@/engines/types'

const { startListening } = useDevice()
const appWindow = getCurrentWebviewWindow()
const { modelSize: l2dSize, handleLoad, handleDestroy, handleResize, handleKeyChange: l2dKeyChange } = useModel()
const catStore = useCatStore()
const { getSharedMenu } = useSharedMenu()
const modelStore = useModelStore()
const model3dStore = useModel3DStore()
const generalStore = useGeneralStore()
const resizing = ref(false)
const switching = ref(false)
const backgroundImagePath = ref<string>()
const { stickActive } = useGamepad()

let engine: DisplayEngine | null = null
let currentModelSize: ModelSize | null = null

onMounted(() => {
  startListening()
  if (catStore.displayMode === '2d') {
    initL2D()
  } else {
    init3D()
  }
})

onUnmounted(() => {
  engine?.destroy()
  engine = null
})

// ── 2D (Live2D) ────────────────────────────────────

function initL2D() {
  engine = new Live2DEngine()
  // L2D uses existing model loading via modelStore watcher below
}

const debouncedResize = useDebounceFn(async () => {
  if (catStore.displayMode === '2d') {
    await handleResize()
  }
  resizing.value = false
}, 100)

useEventListener('resize', () => {
  resizing.value = true
  debouncedResize()
})

watch(() => modelStore.currentModel, async (model) => {
  if (!model || catStore.displayMode !== '2d') return

  handleLoad()
  const path = join(model.path, 'resources', 'background.png')
  const existed = await exists(path)
  backgroundImagePath.value = existed ? convertFileSrc(path) : void 0
  clearObject([modelStore.supportKeys, modelStore.pressedKeys])

  const resourcePath = join(model.path, 'resources')
  const groups = ['left-keys', 'right-keys']
  for await (const groupName of groups) {
    const groupDir = join(resourcePath, groupName)
    const files = await readDir(groupDir).catch(() => [])
    const imageFiles = files.filter(file => isImage(file.name))
    for (const file of imageFiles) {
      const fileName = file.name.split('.')[0]
      modelStore.supportKeys[fileName] = join(groupDir, file.name)
    }
  }
}, { deep: true, immediate: true })

// 2D window sizing
watch([() => catStore.window.scale, l2dSize], async ([scale, size]) => {
  if (!size || catStore.displayMode !== '2d') return
  const { width, height } = size
  appWindow.setSize(new PhysicalSize({
    width: Math.round(width * (scale / 100)),
    height: Math.round(height * (scale / 100)),
  }))
}, { immediate: true })

// 2D keyboard
watch([modelStore.pressedKeys, stickActive], ([keys, stickActive]) => {
  if (catStore.displayMode !== '2d') return
  const dirs = Object.values(keys).map(p => nth(p.split(sep()), -2)!)
  const hasLeft = dirs.some(d => d.startsWith('left'))
  const hasRight = dirs.some(d => d.startsWith('right'))
  l2dKeyChange(true, stickActive.left || hasLeft)
  l2dKeyChange(false, stickActive.right || hasRight)
}, { deep: true })

// ── 3D ─────────────────────────────────────────────

async function init3D() {
  switching.value = true
  backgroundImagePath.value = undefined
  try {
    const pmxPath = model3dStore.currentPmxPath
    if (pmxPath) {
      engine?.destroy()
      engine = new ThreeEngine()
      currentModelSize = await engine.load(pmxPath)
    }
  } catch (e) {
    console.error('[main] 3D init failed:', e)
  } finally {
    switching.value = false
  }
}

// 3D window sizing
watch(() => catStore.window.scale, (scale) => {
  if (catStore.displayMode !== '3d' || !currentModelSize) return
  const { width, height } = currentModelSize
  appWindow.setSize(new PhysicalSize({
    width: Math.round(width * (scale / 100)),
    height: Math.round(height * (scale / 100)),
  }))
})

// ── Mode switching ─────────────────────────────────

watch(() => catStore.displayMode, async (mode) => {
  engine?.destroy()
  engine = null
  currentModelSize = null
  if (mode === '2d') {
    initL2D()
    // Reload current L2D model if available
    if (modelStore.currentModel) handleLoad()
  } else {
    await init3D()
  }
})

// ── Shared watchers ────────────────────────────────

watch(() => catStore.window.visible, (value) => {
  value ? showWindow() : hideWindow()
}, { immediate: true })

watch(() => catStore.window.passThrough, (value) => {
  appWindow.setIgnoreCursorEvents(value)
}, { immediate: true })

watch(() => catStore.window.alwaysOnTop, setAlwaysOnTop, { immediate: true })

watch(() => generalStore.app.taskbarVisible, setTaskbarVisibility, { immediate: true })

function handleMouseDown() {
  appWindow.startDragging()
}

async function handleContextmenu(event: MouseEvent) {
  event.preventDefault()
  if (event.shiftKey) return
  const menu = await getSharedMenu()
  menu.popup()
}

function handleMouseMove(event: MouseEvent) {
  const { buttons, shiftKey, movementX, movementY } = event
  if (buttons !== 2 || !shiftKey) return
  const delta = (movementX + movementY) * 0.5
  const nextScale = Math.max(10, Math.min(catStore.window.scale + delta, 500))
  catStore.window.scale = round(nextScale)
}
</script>

<template>
  <div
    class="relative size-screen overflow-hidden children:(absolute size-full)"
    :class="{ '-scale-x-100': catStore.model.mirror && catStore.displayMode === '2d' }"
    :style="{
      opacity: catStore.window.opacity / 100,
      borderRadius: `${catStore.window.radius}%`,
    }"
    @contextmenu="handleContextmenu"
    @mousedown="handleMouseDown"
    @mousemove="handleMouseMove"
  >
    <!-- L2D background -->
    <img
      v-if="catStore.displayMode === '2d' && backgroundImagePath"
      class="object-cover"
      :src="backgroundImagePath"
    >

    <!-- Shared canvas -->
    <canvas id="live2dCanvas" />

    <!-- L2D key overlays -->
    <img
      v-for="path in catStore.displayMode === '2d' ? modelStore.pressedKeys : {}"
      :key="path"
      class="object-cover"
      :src="convertFileSrc(path)"
    >

    <!-- Resize overlay (2D) -->
    <div
      v-show="catStore.displayMode === '2d' && resizing"
      class="flex items-center justify-center bg-black"
    >
      <span class="text-center text-10vw text-white">
        {{ $t('pages.main.hints.redrawing') }}
      </span>
    </div>

    <!-- Switching overlay -->
    <div
      v-if="switching"
      class="flex items-center justify-center bg-black/50"
    >
      <div class="flex flex-col items-center gap-3 text-white">
        <div class="size-8 border-3 border-white/20 border-t-white rounded-full animate-spin" />
        <span class="text-sm">切换中...</span>
      </div>
    </div>
  </div>
</template>
