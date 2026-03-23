<script setup lang="ts">
import {
  DeleteOutlined,
  FolderOpenOutlined,
  SoundOutlined,
} from '@ant-design/icons-vue'
import { open } from '@tauri-apps/plugin-dialog'
import { Button, InputNumber, List, message, Slider, Switch } from 'ant-design-vue'
import { storeToRefs } from 'pinia'
import { useI18n } from 'vue-i18n'

import ProList from '@/components/pro-list/index.vue'
import ProListItem from '@/components/pro-list-item/index.vue'
import { useDrinkWaterStore } from '@/stores/drinkWater'

const { t } = useI18n()
const store = useDrinkWaterStore()
const { enabled, interval, notificationEnabled, soundEnabled, soundPaths, volume } = storeToRefs(store)

async function addSoundFiles() {
  try {
    const selected = await open({
      multiple: true,
      filters: [{
        name: 'Audio',
        extensions: ['mp3', 'wav', 'ogg', 'aac', 'flac', 'm4a', 'wma'],
      }],
    })

    if (selected) {
      const files = Array.isArray(selected) ? selected : [selected]
      // Append new files to existing list
      soundPaths.value.push(...files)
      message.success(`成功添加 ${files.length} 个音频文件`)
    }
  } catch (err) {
    console.error('Failed to select files:', err)
  }
}

function removeSoundFile(index: number) {
  soundPaths.value.splice(index, 1)
}

function testAlert() {
  store.sendAlert()
}

function formatFileName(path: string) {
  return path.split(/[/\\]/).pop() || path
}
</script>

<template>
  <div class="drink-water-container">
    <ProList :title="t('pages.comprehensive_function.drinkWater.title')">
      <!-- Enable -->
      <ProListItem
        :description="t('pages.comprehensive_function.drinkWater.enableHint')"
        :title="t('pages.comprehensive_function.drinkWater.enable')"
      >
        <Switch v-model:checked="enabled" />
      </ProListItem>

      <!-- Interval -->
      <ProListItem
        v-if="enabled"
        :description="t('pages.comprehensive_function.drinkWater.intervalHint')"
        :title="t('pages.comprehensive_function.drinkWater.interval')"
      >
        <InputNumber
          v-model:value="interval"
          addon-after="min"
          :max="1440"
          :min="1"
          :step="5"
        />
      </ProListItem>

      <!-- Notification -->
      <ProListItem
        v-if="enabled"
        :title="t('pages.comprehensive_function.drinkWater.notification')"
      >
        <Switch v-model:checked="notificationEnabled" />
      </ProListItem>
    </ProList>

    <!-- Sound Settings -->
    <ProList
      v-if="enabled"
      :title="t('pages.comprehensive_function.drinkWater.sound')"
    >
      <!-- Sound Enable -->
      <ProListItem :title="t('pages.comprehensive_function.drinkWater.sound')">
        <Switch v-model:checked="soundEnabled" />
      </ProListItem>

      <template v-if="soundEnabled">
        <!-- Volume -->
        <ProListItem
          :title="t('pages.comprehensive_function.drinkWater.volume')"
          vertical
        >
          <div class="flex items-center gap-3">
            <SoundOutlined class="text-color-3" />
            <Slider
              v-model:value="volume"
              class="flex-1"
              :max="100"
              :min="0"
            />
            <span class="w-10 text-right text-xs text-color-3">{{ volume }}%</span>
          </div>
        </ProListItem>

        <!-- Sound Files -->
        <ProListItem
          :title="t('pages.comprehensive_function.drinkWater.selectSound')"
          vertical
        >
          <div class="mb-2 flex gap-2">
            <Button
              size="small"
              type="primary"
              @click="addSoundFiles"
            >
              <template #icon>
                <FolderOpenOutlined />
              </template>
              {{ t('pages.comprehensive_function.drinkWater.browse') }}
            </Button>
            <Button
              size="small"
              @click="testAlert"
            >
              {{ t('pages.comprehensive_function.drinkWater.test') }}
            </Button>
          </div>

          <List
            bordered
            class="rounded-lg bg-white dark:bg-[#1f1f1f]"
            :data-source="soundPaths"
            size="small"
          >
            <template #renderItem="{ item, index }">
              <List.Item>
                <div class="group w-full flex items-center justify-between">
                  <span
                    class="mr-2 flex-1 truncate text-xs"
                    :title="item"
                  >{{ formatFileName(item) }}</span>
                  <Button
                    class="opacity-0 transition-opacity group-hover:opacity-100"
                    danger
                    size="small"
                    type="text"
                    @click="removeSoundFile(index)"
                  >
                    <template #icon>
                      <DeleteOutlined />
                    </template>
                  </Button>
                </div>
              </List.Item>
            </template>
          </List>
          <div
            v-if="soundPaths.length === 0"
            class="mt-1 text-xs text-color-3"
          >
            未选择音频文件
          </div>
        </ProListItem>
      </template>
    </ProList>
  </div>
</template>
