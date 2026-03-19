<script setup lang="ts">
// import { emit } from '@tauri-apps/api/event'
// import { info } from '@tauri-apps/plugin-log'
import { storeToRefs } from 'pinia'
import { nextTick, reactive, watch } from 'vue'

import ProList from '@/components/pro-list/index.vue'
import ProShortcut from '@/components/pro-shortcut/index.vue'
// import { useModel } from '@/composables/useModel'
import { useModelDataSync } from '@/composables/useModelDataSync'
// import { useTauriListen } from '@/composables/useTauriListen'
import { useTauriShortcut } from '@/composables/useTauriShortcut'
import { toggleWindowVisible } from '@/plugins/window'
import { useCatStore } from '@/stores/cat'
import { useShortcutStore } from '@/stores/shortcut.ts'

const shortcutStore = useShortcutStore()
const { visibleCat, visiblePreference, mirrorMode, penetrable, alwaysOnTop } = storeToRefs(shortcutStore)
const catStore = useCatStore()

useTauriShortcut(visibleCat, () => {
  catStore.window.visible = !catStore.window.visible
})

useTauriShortcut(visiblePreference, () => {
  toggleWindowVisible('preference')
})

useTauriShortcut(mirrorMode, () => {
  catStore.model.mirror = !catStore.model.mirror
})

useTauriShortcut(penetrable, () => {
  catStore.window.passThrough = !catStore.window.passThrough
})

useTauriShortcut(alwaysOnTop, () => {
  catStore.window.alwaysOnTop = !catStore.window.alwaysOnTop
})

// 使用模型数据同步hook
const { currentModel, expressions, isModelDataLoaded } = useModelDataSync()

// 使用响应式对象存储表情快捷键
const expressionShortcuts = reactive<Record<string, string>>({})

// 初始化表情快捷键
async function initializeExpressionShortcuts() {
  await nextTick()

  // 清空现有数据
  Object.keys(expressionShortcuts).forEach((key) => {
    delete expressionShortcuts[key]
  })

  if (!currentModel.value?.id || !expressions.value.length) return

  // 为每个表情设置快捷键值
  expressions.value.forEach((expression) => {
    const hotkey = shortcutStore.getExpressionHotkey(currentModel.value!.id, expression.Name)
    expressionShortcuts[expression.Name] = hotkey || ''
  })
}

// 保存表情快捷键
async function saveExpressionShortcut(expressionName: string, hotkey: string) {
  if (!currentModel.value?.id) return

  if (hotkey) {
    shortcutStore.addExpressionShortcut(
      currentModel.value.id,
      expressionName,
      hotkey,
    )
  } else {
    shortcutStore.removeExpressionShortcut(currentModel.value.id, expressionName)
  }

  // 通知主窗口快捷键变更
  const { emit } = await import('@tauri-apps/api/event')
  await emit('expression-shortcut-changed', {
    modelId: currentModel.value.id,
    expressionName,
    hotkey,
  })
}

// 监听表情快捷键变化
function setupExpressionWatchers() {
  if (!expressions.value.length) return

  expressions.value.forEach((expression) => {
    watch(
      () => expressionShortcuts[expression.Name],
      (newHotkey, oldHotkey) => {
        // 避免初始化时触发保存
        if (oldHotkey !== undefined) {
          saveExpressionShortcut(expression.Name, newHotkey || '')
        }
      },
    )
  })
}

// 监听模型和表情变化
watch([() => currentModel.value?.id, () => expressions.value.length], async () => {
  await initializeExpressionShortcuts()
  setupExpressionWatchers()
}, { immediate: true })
</script>

<template>
  <ProList :title="$t('pages.preference.shortcut.title')">
    <ProShortcut
      v-model="shortcutStore.visibleCat"
      :description="$t('pages.preference.shortcut.hints.toggleCat')"
      :title="$t('pages.preference.shortcut.labels.toggleCat')"
    />

    <ProShortcut
      v-model="shortcutStore.visiblePreference"
      :description="$t('pages.preference.shortcut.hints.togglePreferences')"
      :title="$t('pages.preference.shortcut.labels.togglePreferences')"
    />

    <ProShortcut
      v-model="shortcutStore.mirrorMode"
      :description="$t('pages.preference.shortcut.hints.mirrorMode')"
      :title="$t('pages.preference.shortcut.labels.mirrorMode')"
    />

    <ProShortcut
      v-model="shortcutStore.penetrable"
      :description="$t('pages.preference.shortcut.hints.passThrough')"
      :title="$t('pages.preference.shortcut.labels.passThrough')"
    />

    <ProShortcut
      v-model="shortcutStore.alwaysOnTop"
      :description="$t('pages.preference.shortcut.hints.alwaysOnTop')"
      :title="$t('pages.preference.shortcut.labels.alwaysOnTop')"
    />
  </ProList>
  <!-- 表情快捷键配置 -->
  <ProList
    v-if="isModelDataLoaded && currentModel && expressions.length > 0"
    title="表情快捷键"
  >
    <ProShortcut
      v-for="expression in expressions"
      :key="`${currentModel.id}-${expression.Name}`"
      v-model="expressionShortcuts[expression.Name]"
      :description="`为 ${expression.Name || '未命名表情'} 设置快捷键`"
      :title="expression.Name || '未命名表情'"
    />
  </ProList>

  <ProList
    v-else-if="isModelDataLoaded && currentModel"
    title="表情快捷键"
  >
    <div class="empty-state">
      当前模型没有表情
    </div>
  </ProList>

  <ProList
    v-else-if="!isModelDataLoaded"
    title="表情快捷键"
  >
    <div class="empty-state">
      正在加载模型数据...
    </div>
  </ProList>
</template>

<style scoped>
.empty-state {
  text-align: center;
  color: #999;
  padding: 32px;
  font-size: 14px;
}
</style>
