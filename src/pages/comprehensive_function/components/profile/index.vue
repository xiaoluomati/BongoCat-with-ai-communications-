<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Button, Input, message, Modal, Tag } from 'ant-design-vue'
import ProList from '@/components/pro-list/index.vue'
import ProListItem from '@/components/pro-list-item/index.vue'

interface Interaction {
  date: string
  activity: string
  summary: string
}

interface SpecialMemory {
  title: string
  description: string
  date: string
  tags: string[]
}

interface Profile {
  user_name: string | null
  traits: string[]
  preferences: Record<string, string>
  important_dates: Record<string, string>
  recent_interactions: Interaction[]
  special_memories: SpecialMemory[]
  conversation_count: number
  last_updated: string
}

const loading = ref(false)
const profileData = ref<Profile>({
  user_name: null,
  traits: [],
  preferences: {},
  important_dates: {},
  recent_interactions: [],
  special_memories: [],
  conversation_count: 0,
  last_updated: '',
})

const isModalOpen = ref(false)
const editForm = ref<Profile>({} as Profile)
const newTrait = ref('')
const newPrefKey = ref('')
const newPrefValue = ref('')
const newDateKey = ref('')
const newDateValue = ref('')

async function loadProfile() {
  loading.value = true
  try {
    const data = await invoke<any>('get_user_profile')
    profileData.value = {
      user_name: data.user_name || null,
      traits: data.traits || [],
      preferences: data.preferences || {},
      important_dates: data.important_dates || {},
      recent_interactions: data.recent_interactions || [],
      special_memories: data.special_memories || [],
      conversation_count: data.conversation_count || 0,
      last_updated: data.last_updated || '',
    }
  } catch (e) {
    console.error(e)
  } finally {
    loading.value = false
  }
}

function openEdit() {
  editForm.value = JSON.parse(JSON.stringify(profileData.value))
  newTrait.value = ''
  newPrefKey.value = ''
  newPrefValue.value = ''
  newDateKey.value = ''
  newDateValue.value = ''
  isModalOpen.value = true
}

async function saveProfile() {
  try {
    await invoke('save_user_profile', { profile: editForm.value })
    profileData.value = JSON.parse(JSON.stringify(editForm.value))
    message.success('保存成功')
    isModalOpen.value = false
  } catch (e) {
    console.error(e)
    message.error('保存失败')
  }
}

function addTrait() {
  const v = newTrait.value.trim()
  if (v && editForm.value.traits) {
    editForm.value.traits.push(v)
    newTrait.value = ''
  }
}

function removeTrait(i: number) {
  editForm.value.traits?.splice(i, 1)
}

function addPreference() {
  if (newPrefKey.value.trim() && newPrefValue.value.trim()) {
    editForm.value.preferences = editForm.value.preferences || {}
    editForm.value.preferences[newPrefKey.value.trim()] = newPrefValue.value.trim()
    newPrefKey.value = ''
    newPrefValue.value = ''
  }
}

function removePreference(key: string) {
  delete editForm.value.preferences?.[key]
}

function addImportantDate() {
  if (newDateKey.value.trim() && newDateValue.value.trim()) {
    editForm.value.important_dates = editForm.value.important_dates || {}
    editForm.value.important_dates[newDateKey.value.trim()] = newDateValue.value.trim()
    newDateKey.value = ''
    newDateValue.value = ''
  }
}

function removeImportantDate(key: string) {
  delete editForm.value.important_dates?.[key]
}

function fmtDate(s: string) {
  return s ? new Date(s).toLocaleString('zh-CN') : '从未更新'
}

function getPreferencesText(prefs: Record<string, string>) {
  if (!prefs || Object.keys(prefs).length === 0) return '暂无'
  return Object.entries(prefs).map(([k, v]) => `${k}: ${v}`).join('， ')
}

function getDatesText(dates: Record<string, string>) {
  if (!dates || Object.keys(dates).length === 0) return '暂无'
  return Object.entries(dates).map(([k, v]) => `${k}: ${v}`).join('， ')
}

onMounted(loadProfile)
</script>

<template>
  <ProList :loading="loading">
    <!-- Header -->
    <div class="mb-4 flex items-center justify-between">
      <div></div>
      <Button type="primary" size="small" @click="openEdit">编辑</Button>
    </div>

    <!-- Basic Info -->
    <ProListItem title="用户名" :description="profileData.user_name || '未设置'" />
    <ProListItem title="对话轮数" :description="String(profileData.conversation_count || 0)" />
    <ProListItem title="上次更新" :description="fmtDate(profileData.last_updated)" />

    <!-- Traits -->
    <ProListItem title="性格特点">
      <div class="flex flex-wrap gap-1 mt-1">
        <Tag v-for="(t, i) in profileData.traits" :key="i" color="purple">{{ t }}</Tag>
        <span v-if="!profileData.traits?.length" class="text-gray-400">暂无</span>
      </div>
    </ProListItem>

    <!-- Preferences -->
    <ProListItem title="偏好">
      <div class="text-sm mt-1">
        {{ getPreferencesText(profileData.preferences) }}
      </div>
    </ProListItem>

    <!-- Important Dates -->
    <ProListItem title="重要日期">
      <div class="text-sm mt-1">
        {{ getDatesText(profileData.important_dates) }}
      </div>
    </ProListItem>

    <!-- Recent Interactions (只读，由AI自动更新) -->
    <ProListItem title="最近互动">
      <div v-if="profileData.recent_interactions?.length" class="space-y-2 mt-1">
        <div v-for="(item, i) in profileData.recent_interactions.slice(0, 3)" :key="i" class="text-sm bg-gray-50 p-2 rounded">
          <div class="text-gray-500">{{ item.date }} - {{ item.activity }}</div>
          <div>{{ item.summary }}</div>
        </div>
      </div>
      <span v-else class="text-gray-400">暂无</span>
    </ProListItem>

    <!-- Special Memories (只读，由AI自动更新) -->
    <ProListItem title="专属回忆">
      <div v-if="profileData.special_memories?.length" class="space-y-2 mt-1">
        <div v-for="(mem, i) in profileData.special_memories.slice(0, 3)" :key="i" class="text-sm bg-purple-50 p-2 rounded">
          <div class="font-medium">{{ mem.title }}</div>
          <div class="text-gray-600">{{ mem.description }}</div>
        </div>
      </div>
      <span v-else class="text-gray-400">暂无</span>
    </ProListItem>
  </ProList>

  <!-- Edit Modal -->
  <Modal v-model:open="isModalOpen" title="编辑用户画像" width="600" @ok="saveProfile">
    <div class="space-y-4 max-h-[60vh] overflow-y-auto">
      <!-- User Name -->
      <div>
        <div class="text-sm font-medium mb-1">用户名</div>
        <Input v-model:value="editForm.user_name" placeholder="你的名字" />
      </div>

      <!-- Traits -->
      <div>
        <div class="text-sm font-medium mb-1">性格特点</div>
        <div class="flex flex-wrap gap-1 mb-2">
          <Tag v-for="(t, i) in editForm.traits" :key="i" closable @close="removeTrait(i)">{{ t }}</Tag>
        </div>
        <div class="flex gap-2">
          <Input v-model:value="newTrait" placeholder="输入后回车添加" @keydown.enter="addTrait" class="flex-1" />
          <Button @click="addTrait">+</Button>
        </div>
      </div>

      <!-- Preferences -->
      <div>
        <div class="text-sm font-medium mb-1">偏好设置</div>
        <div v-if="editForm.preferences" class="space-y-1 mb-2">
          <div v-for="(v, k) in editForm.preferences" :key="k" class="flex items-center gap-2">
            <span class="text-sm flex-1">{{ k }}: {{ v }}</span>
            <Button size="small" danger @click="removePreference(k as string)">×</Button>
          </div>
        </div>
        <div class="flex gap-2">
          <Input v-model:value="newPrefKey" placeholder="如：喜欢音乐" class="flex-1" />
          <Input v-model:value="newPrefValue" placeholder="如：古典音乐" class="flex-1" />
          <Button @click="addPreference">+</Button>
        </div>
      </div>

      <!-- Important Dates -->
      <div>
        <div class="text-sm font-medium mb-1">重要日期</div>
        <div v-if="editForm.important_dates" class="space-y-1 mb-2">
          <div v-for="(v, k) in editForm.important_dates" :key="k" class="flex items-center gap-2">
            <span class="text-sm flex-1">{{ k }}: {{ v }}</span>
            <Button size="small" danger @click="removeImportantDate(k as string)">×</Button>
          </div>
        </div>
        <div class="flex gap-2">
          <Input v-model:value="newDateKey" placeholder="如：生日" class="flex-1" />
          <Input v-model:value="newDateValue" placeholder="如：06-15" class="flex-1" />
          <Button @click="addImportantDate">+</Button>
        </div>
      </div>

      <!-- 只读提示 -->
      <div class="text-sm text-gray-400 mt-4">
        最近互动和专属回忆由AI根据对话自动分析更新，无需手动编辑。
      </div>
    </div>
  </Modal>
</template>
