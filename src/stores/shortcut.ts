// import { info } from '@tauri-apps/plugin-log'
import { defineStore } from 'pinia'
import { ref } from 'vue'

export type HotKey = 'visibleCat' | 'mirrorMode' | 'penetrable' | 'alwaysOnTop'
export interface ExpressionShortcut {
  expressionName: string
  hotkey: string
}
export const useShortcutStore = defineStore('shortcut', () => {
  const visibleCat = ref('')
  const visiblePreference = ref('')
  const mirrorMode = ref('')
  const penetrable = ref('')
  const alwaysOnTop = ref('')
  // 新增：表情快捷键配置，使用pinia持久化插件自动保存
  const expressionShortcuts = ref<Record<string, ExpressionShortcut[]>>({})

  const addExpressionShortcut = (modelId: string, expressionName: string, hotkey: string) => {
    if (!expressionShortcuts.value[modelId]) {
      expressionShortcuts.value[modelId] = []
    }

    const existingIndex = expressionShortcuts.value[modelId].findIndex(
      s => s.expressionName === expressionName,
    )

    if (existingIndex >= 0) {
      expressionShortcuts.value[modelId][existingIndex].hotkey = hotkey
    } else {
      expressionShortcuts.value[modelId].push({
        expressionName,
        hotkey,
      })
    }
  }

  const removeExpressionShortcut = (modelId: string, expressionName: string) => {
    if (!expressionShortcuts.value[modelId]) return

    const index = expressionShortcuts.value[modelId].findIndex(
      s => s.expressionName === expressionName,
    )

    if (index >= 0) {
      expressionShortcuts.value[modelId].splice(index, 1)
    }
  }

  const getModelExpressionShortcuts = (modelId: string): ExpressionShortcut[] => {
    return expressionShortcuts.value[modelId] || []
  }

  const getExpressionHotkey = (modelId: string, expressionName: string): string => {
    const shortcuts = expressionShortcuts.value[modelId] || []
    const found = shortcuts.find(s => s.expressionName === expressionName)
    return found?.hotkey || ''
  }

  return {
    visibleCat,
    visiblePreference,
    mirrorMode,
    penetrable,
    alwaysOnTop,
    expressionShortcuts,
    addExpressionShortcut,
    removeExpressionShortcut,
    getModelExpressionShortcuts,
    getExpressionHotkey,
  }
}, {
  tauri: {
    filterKeys: ['visibleCat', 'visiblePreference', 'mirrorMode', 'penetrable', 'alwaysOnTop', 'expressionShortcuts'],
    filterKeysStrategy: 'pick',
  },
})
