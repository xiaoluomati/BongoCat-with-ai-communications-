import type { ShortcutHandler } from '@tauri-apps/plugin-global-shortcut'

import { emit } from '@tauri-apps/api/event'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { nextTick, ref, watch } from 'vue'

import { useTauriListen } from './useTauriListen'
import { useTauriShortcut } from './useTauriShortcut'

import { LISTEN_KEY } from '@/constants'
import { useModelStore } from '@/stores/model'
import { useShortcutStore } from '@/stores/shortcut'
import live2d from '@/utils/live2d'

export function useExpressionShortcuts() {
  const modelStore = useModelStore()
  const shortcutStore = useShortcutStore()
  const appWindow = getCurrentWebviewWindow()

  const registeredShortcuts = ref<Array<{ shortcut: string, cleanup: () => void }>>([])

  const registerCurrentModelShortcuts = async () => {
    await nextTick()
    clearRegisteredShortcuts()

    if (!modelStore.currentModel?.id || !modelStore.expressions.length) return

    const shortcuts = shortcutStore.getModelExpressionShortcuts(modelStore.currentModel.id)

    shortcuts.forEach((shortcutConfig) => {
      if (shortcutConfig.hotkey) {
        const shortcutRef = ref(shortcutConfig.hotkey)
        const handler: ShortcutHandler = () => {
          live2d.playExpressions(shortcutConfig.expressionName)
        }

        useTauriShortcut(shortcutRef, handler)

        registeredShortcuts.value.push({
          shortcut: shortcutConfig.hotkey,
          cleanup: () => {
            shortcutRef.value = ''
          },
        })
      }
    })
  }

  const clearRegisteredShortcuts = () => {
    registeredShortcuts.value.forEach(({ cleanup }) => {
      cleanup()
    })
    registeredShortcuts.value = []
  }

  const saveExpressionShortcut = async (modelId: string, expressionName: string, hotkey: string) => {
    shortcutStore.addExpressionShortcut(modelId, expressionName, hotkey)

    await emit(LISTEN_KEY.EXPRESSION_SHORTCUT_CHANGED, {
      modelId,
      expressionName,
      hotkey,
    })
  }

  const removeExpressionShortcut = async (modelId: string, expressionName: string) => {
    shortcutStore.removeExpressionShortcut(modelId, expressionName)

    await emit(LISTEN_KEY.EXPRESSION_SHORTCUT_CHANGED, {
      modelId,
      expressionName,
      hotkey: '',
    })
  }

  // 主窗口：同步模型数据给偏好窗口
  if (appWindow.label === 'main') {
    // 监听偏好窗口请求模型数据
    useTauriListen(LISTEN_KEY.REQUEST_MODEL_DATA, async () => {
      await emit(LISTEN_KEY.MODEL_DATA_SYNC, {
        currentModel: modelStore.currentModel,
        expressions: modelStore.expressions,
      })
    })

    // 监听模型变化，主动同步数据
    watch(
      [() => modelStore.currentModel?.id, () => modelStore.expressions.length],
      async () => {
        registerCurrentModelShortcuts()

        // 同步数据到偏好窗口
        await emit(LISTEN_KEY.MODEL_DATA_SYNC, {
          currentModel: modelStore.currentModel,
          expressions: modelStore.expressions,
        })
      },
      { immediate: true, flush: 'post' },
    )

    // 监听表情快捷键变更
    useTauriListen(LISTEN_KEY.EXPRESSION_SHORTCUT_CHANGED, () => {
      registerCurrentModelShortcuts()
    })
  }

  return {
    registerCurrentModelShortcuts,
    clearRegisteredShortcuts,
    saveExpressionShortcut,
    removeExpressionShortcut,
  }
}
