<script setup lang="ts">
import { DeleteOutlined, EditOutlined, PlusOutlined, SwapOutlined } from '@ant-design/icons-vue'
import { Button, Card, Input, Modal, message, Select, Spin, Switch } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import { ref, onMounted, computed } from 'vue'

import ProList from '@/components/pro-list/index.vue'
import ProListItem from '@/components/pro-list-item/index.vue'

// Types
interface Character {
  id: string
  name: string
  description: string
  preset_prompt: string
  system_prompt: string
  avatar: string
  preferred_address: string
  voice_id?: string
}

interface CharacterBrief {
  id: string
  name: string
  description: string
  avatar: string
  preferred_address: string
}

// State
const loading = ref(false)
const characters = ref<CharacterBrief[]>([])
const currentCharacterId = ref('')
const editingCharacter = ref<Character | null>(null)
const isModalVisible = ref(false)
const isEditing = ref(false)
const ttsVoices = ref<Record<string, { speaker: string; emo: string }>>({})
const ttsDefaultVoiceId = ref('')
const loadingVoices = ref(false)

// Default character template
const defaultCharacter: Character = {
  id: '',
  name: '',
  description: '',
  preset_prompt: '你是一只活泼可爱的小猫咪...',
  system_prompt: '',
  avatar: '',
  preferred_address: '亲爱的',
}

// Load characters
async function loadCharacters() {
  loading.value = true
  try {
    characters.value = await invoke<CharacterBrief[]>('list_characters')
    currentCharacterId.value = await invoke<string>('get_current_character')
  } catch (err) {
    console.error('Failed to load characters:', err)
    message.error('加载角色失败')
  } finally {
    loading.value = false
  }
}

// Load TTS voices
async function loadTTSVoices() {
  loadingVoices.value = true
  try {
    const config = await invoke<any>('get_tts_config')
    ttsVoices.value = config.voices || {}
    ttsDefaultVoiceId.value = config.default_voice_id || ''
  } catch (err) {
    console.error('Failed to load TTS voices:', err)
    ttsVoices.value = {}
  } finally {
    loadingVoices.value = false
  }
}

// Open create modal
function openCreateModal() {
  loadTTSVoices()
  editingCharacter.value = { ...defaultCharacter }
  isEditing.value = false
  isModalVisible.value = true
}

// Open edit modal
async function openEditModal(char: CharacterBrief) {
  console.warn('Opening edit modal for:', char)
  await loadTTSVoices()
  try {
    const charData = await invoke<Character>('load_character', { id: char.id })
    console.warn('Loaded character data:', charData)
    editingCharacter.value = charData
    isEditing.value = true
    isModalVisible.value = true
  } catch (err) {
    console.error('Failed to load character:', err)
    message.error('加载角色详情失败: ' + err)
  }
}

// Save character
async function saveCharacter() {
  if (!editingCharacter.value) return
  
  // Validation
  if (!editingCharacter.value.id.trim()) {
    message.error('请输入角色ID')
    return
  }
  if (!editingCharacter.value.name.trim()) {
    message.error('请输入角色名称')
    return
  }
  
  // Generate ID from name if empty
  if (!editingCharacter.value.id) {
    editingCharacter.value.id = editingCharacter.value.name.toLowerCase().replace(/\s+/g, '_')
  }
  
  try {
    await invoke('save_character', { character: editingCharacter.value })
    message.success(isEditing.value ? '角色已更新' : '角色已创建')
    isModalVisible.value = false
    await loadCharacters()
  } catch (err) {
    console.error('Failed to save character:', err)
    message.error('保存角色失败')
  }
}

// Delete character
async function deleteCharacter(id: string) {
  if (id === 'cat') {
    message.error('不能删除默认角色')
    return
  }
  
  Modal.confirm({
    title: '确认删除',
    content: '确定要删除这个角色吗？此操作不可恢复。',
    async onOk() {
      try {
        await invoke('delete_character', { id })
        message.success('角色已删除')
        await loadCharacters()
      } catch (err) {
        console.error('Failed to delete character:', err)
        message.error('删除角色失败: ' + err)
      }
    },
  })
}

// Switch character
async function switchCharacter(id: string) {
  try {
    await invoke('switch_character', { id })
    currentCharacterId.value = id
    message.success('已切换到 ' + characters.value.find(c => c.id === id)?.name)
  } catch (err) {
    console.error('Failed to switch character:', err)
    message.error('切换角色失败')
  }
}

// Check if character is current
const isCurrentCharacter = (id: string) => id === currentCharacterId.value

onMounted(() => {
  loadCharacters()
})
</script>

<template>
  <ProList :loading="loading">
    <!-- Header -->
    <div class="mb-4 flex items-center justify-between">
      <div class="text-lg font-medium"></div>
      <Button type="primary" @click="openCreateModal">
        <template #icon><PlusOutlined /></template>
        新建角色
      </Button>
    </div>

    <!-- Character List -->
    <ProListItem
      v-for="char in characters"
      :key="char.id"
      class="!bg-white/80 dark:!bg-black/20"
    >
      <div class="flex items-center justify-between w-full">
        <div class="flex items-center gap-3">
          <!-- Avatar -->
          <div class="w-12 h-12 rounded-full bg-gradient-to-br from-blue-400 to-purple-500 flex items-center justify-center text-white text-lg font-bold">
            {{ char.name.charAt(0).toUpperCase() }}
          </div>
          
          <!-- Info -->
          <div>
            <div class="flex items-center gap-2">
              <span class="font-medium">{{ char.name }}</span>
              <span v-if="isCurrentCharacter(char.id)" class="text-xs px-2 py-0.5 bg-green-100 text-green-600 rounded">
                当前
              </span>
            </div>
            <div class="text-sm text-gray-500">{{ char.description || '暂无描述' }}</div>
          </div>
        </div>
        
        <!-- Actions -->
        <div class="flex items-center gap-2">
          <Button 
            v-if="!isCurrentCharacter(char.id)"
            size="small"
            @click="switchCharacter(char.id)"
          >
            <template #icon><SwapOutlined /></template>
            切换
          </Button>
          <Button size="small" @click="openEditModal(char)">
            <template #icon><EditOutlined /></template>
            编辑
          </Button>
          <Button 
            v-if="char.id !== 'cat'"
            size="small" 
            danger
            @click="deleteCharacter(char.id)"
          >
            <template #icon><DeleteOutlined /></template>
          </Button>
        </div>
      </div>
    </ProListItem>

    <!-- Empty State -->
    <div v-if="!loading && characters.length === 0" class="text-center py-8 text-gray-500">
      暂无角色
    </div>
  </ProList>

  <!-- Edit/Create Modal -->
  <Modal
    v-model:open="isModalVisible"
    :title="isEditing ? '编辑角色' : '新建角色'"
    :width="700"
    @ok="saveCharacter"
  >
    <div v-if="editingCharacter" class="space-y-4">
      <!-- Basic Info -->
      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="block text-sm font-medium mb-1">角色ID</label>
          <Input 
            v-model:value="editingCharacter.id" 
            placeholder="唯一标识，如: cat"
            :disabled="isEditing"
          />
        </div>
        <div>
          <label class="block text-sm font-medium mb-1">角色名称</label>
          <Input 
            v-model:value="editingCharacter.name" 
            placeholder="显示名称"
          />
        </div>
      </div>

      <!-- Description -->
      <div>
        <label class="block text-sm font-medium mb-1">角色描述</label>
        <Input 
          v-model:value="editingCharacter.description" 
          placeholder="简短描述"
        />
      </div>

      <!-- Preferred Address -->
      <div>
        <label class="block text-sm font-medium mb-1">
          对用户的称呼
          <span class="text-gray-400 font-normal">(如：亲爱的、宝贝、老公)</span>
        </label>
        <Input 
          v-model:value="editingCharacter.preferred_address" 
          placeholder="亲爱的"
        />
      </div>

      <!-- Voice Selection -->
      <div v-if="Object.keys(ttsVoices).length > 0">
        <label class="block text-sm font-medium mb-1">
          TTS 音色
          <span class="text-gray-400 font-normal">(关联语音合成音色)</span>
        </label>
        <Select
          v-model:value="editingCharacter.voice_id"
          class="w-full"
          placeholder="选择音色（可选）"
          allowClear
        >
          <Select.Option :value="''">使用默认音色</Select.Option>
          <Select.Option v-for="(voice, id) in ttsVoices" :key="id" :value="id">
            {{ id === ttsDefaultVoiceId ? `${id} (默认)` : id }} - {{ voice.speaker }}
          </Select.Option>
        </Select>
      </div>

      <!-- Preset Prompt -->
      <div>
        <label class="block text-sm font-medium mb-1">
          预设提示词
          <span class="text-gray-400 font-normal">(定义角色核心性格)</span>
        </label>
        <Input.TextArea 
          v-model:value="editingCharacter.preset_prompt" 
          :rows="4"
          placeholder="例如: 你是一只活泼可爱的小猫咪，喜欢..."
        />
      </div>

      <!-- System Prompt -->
      <div>
        <label class="block text-sm font-medium mb-1">
          角色定义
          <span class="text-gray-400 font-normal">(补充说明、背景故事等)</span>
        </label>
        <Input.TextArea 
          v-model:value="editingCharacter.system_prompt" 
          :rows="4"
          placeholder="额外的系统提示词内容..."
        />
      </div>
    </div>
  </Modal>
</template>
