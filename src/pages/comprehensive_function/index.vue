<script setup lang="ts">
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { Flex } from 'ant-design-vue'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import DrinkWater from './components/drink_water/index.vue'
import Music from './components/music/index.vue'
import LLM from './components/llm/index.vue'
import Memory from './components/memory/index.vue'
import Character from './components/character/index.vue'
import Profile from './components/profile/index.vue'

import { useAppStore } from '@/stores/app'
import { useGeneralStore } from '@/stores/general'
import { isMac } from '@/utils/platform'

const appStore = useAppStore()
const current = ref(0)
const { t } = useI18n()
const generalStore = useGeneralStore()
const appWindow = getCurrentWebviewWindow()

watch(() => generalStore.appearance.language, () => {
  appWindow.setTitle(t('pages.comprehensive_function.title'))
}, { immediate: true })

const menus = computed(() => [
  {
    label: t('pages.comprehensive_function.music.title'),
    icon: 'i-solar:music-note-linear',
    component: Music,
  },
  {
    label: t('pages.comprehensive_function.drinkWater.title'),
    icon: 'i-solar:cup-linear',
    component: DrinkWater,
  },
  {
    label: 'AI 对话',
    icon: 'i-solar:chat-round-linear',
    component: LLM,
  },
  {
    label: '记忆管理',
    icon: 'i-solar:book-2-linear',
    component: Memory,
  },
  {
    label: '角色管理',
    icon: 'i-solar:users-group-rounded-linear',
    component: Character,
  },
  {
    label: '用户画像',
    icon: 'i-solar:user-id-linear',
    component: Profile,
  },
])
</script>

<template>
  <Flex class="h-screen">
    <div
      class="h-full w-30 flex flex-col items-center gap-4 overflow-auto dark:(bg-color-3 bg-none) bg-gradient-from-primary-1 bg-gradient-to-black/1 bg-gradient-linear"
      :class="[isMac ? 'pt-8' : 'pt-4']"
      data-tauri-drag-region
    >
      <div class="flex flex-col items-center gap-2">
        <div class="b b-color-2 rounded-2xl b-solid">
          <img
            class="size-15"
            data-tauri-drag-region
            src="/logo.png"
          >
        </div>

        <span class="font-bold">{{ appStore.name }}</span>
      </div>

      <div class="flex flex-col gap-2">
        <div
          v-for="(item, index) in menus"
          :key="item.label"
          class="size-20 flex flex-col cursor-pointer items-center justify-center gap-2 rounded-lg hover:bg-color-7 dark:text-color-2 text-color-3 transition"
          :class="{ 'bg-color-2! text-primary-5 dark:text-primary-7 font-bold dark:bg-color-8!': current === index }"
          @click="current = index"
        >
          <div
            class="size-8"
            :class="item.icon"
          />

          <span>{{ item.label }}</span>
        </div>
      </div>
    </div>

    <div
      v-for="(item, index) in menus"
      v-show="current === index"
      :key="item.label"
      class="flex-1 overflow-auto bg-color-8 dark:bg-color-2 p-4"
      data-tauri-drag-region
    >
      <component :is="item.component" />
    </div>
  </Flex>
</template>
