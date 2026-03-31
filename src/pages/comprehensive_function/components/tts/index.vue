<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { Button, Input, InputNumber, message, Modal, Slider, Switch } from 'ant-design-vue'
import { ref, onMounted, computed } from 'vue'

import ProList from '@/components/pro-list/index.vue'
import ProListItem from '@/components/pro-list-item/index.vue'

// TTS Configuration
interface VoiceConfig {
  speaker: string
  emo: string
  weight: number
}

interface TTSConfig {
  enabled: boolean
  base_url: string
  default_voice_id: string
  volume: number
  speed: number
  voices: Record<string, VoiceConfig>
}

const enabled = ref(false)
const baseUrl = ref('http://localhost:9880')
const volume = ref(80)
const speed = ref(1.0)
const voices = ref<Record<string, VoiceConfig>>({})
const defaultVoiceId = ref('suyao')
const testingConnection = ref(false)
const connectionStatus = ref<'unknown' | 'success' | 'failed'>('unknown')

// Voice Modal
const showVoiceModal = ref(false)
const editingVoiceId = ref('')
const voiceForm = ref<VoiceConfig>({
  speaker: '',
  emo: '',
  weight: 1.0
})

const voiceList = computed(() => {
  return Object.entries(voices.value).map(([id, voice]) => ({
    id,
    ...voice
  }))
})

// Load config on mount
onMounted(async () => {
  try {
    const config = await invoke<TTSConfig>('get_tts_config')
    enabled.value = config.enabled
    baseUrl.value = config.base_url
    volume.value = config.volume
    speed.value = config.speed
    voices.value = config.voices || {}
    defaultVoiceId.value = config.default_voice_id
  } catch (err) {
    console.error('Failed to load TTS config:', err)
  }
})

// Save config
async function saveConfig() {
  try {
    const config: TTSConfig = {
      enabled: enabled.value,
      base_url: baseUrl.value,
      default_voice_id: defaultVoiceId.value,
      volume: volume.value,
      speed: speed.value,
      voices: voices.value
    }
    await invoke('save_tts_config', { ttsConfig: config })
    message.success('配置已保存')
  } catch (err) {
    console.error('Failed to save TTS config:', err)
    message.error('保存配置失败')
  }
}

// Test connection
async function testConnection() {
  testingConnection.value = true
  connectionStatus.value = 'unknown'
  
  try {
    await saveConfig()
    // Try to get cache info as a connection test
    await invoke('get_tts_cache_info')
    connectionStatus.value = 'success'
    message.success('连接成功！')
  } catch (err) {
    console.error('Connection test failed:', err)
    connectionStatus.value = 'failed'
    message.error('连接失败: ' + err)
  } finally {
    testingConnection.value = false
  }
}

// Open voice modal for add
function openAddVoiceModal() {
  editingVoiceId.value = ''
  voiceForm.value = {
    speaker: '',
    emo: '',
    weight: 1.0
  }
  showVoiceModal.value = true
}

// Open voice modal for edit
function openEditVoiceModal(voiceId: string) {
  editingVoiceId.value = voiceId
  voiceForm.value = { ...voices.value[voiceId] }
  showVoiceModal.value = true
}

// Save voice
async function saveVoice() {
  if (!editingVoiceId.value) {
    // Add new voice
    const newId = `voice_${Date.now()}`
    voices.value[newId] = { ...voiceForm.value }
    defaultVoiceId.value = newId
  } else {
    // Update existing voice
    voices.value[editingVoiceId.value] = { ...voiceForm.value }
  }
  
  await saveConfig()
  showVoiceModal.value = false
  message.success(editingVoiceId.value ? '音色已更新' : '音色已添加')
}

// Delete voice
async function deleteVoice(voiceId: string) {
  Modal.confirm({
    title: '确认删除',
    content: `确定要删除音色 "${voiceId}" 吗？`,
    okText: '删除',
    cancelText: '取消',
    onOk: async () => {
      delete voices.value[voiceId]
      if (defaultVoiceId.value === voiceId) {
        defaultVoiceId.value = Object.keys(voices.value)[0] || 'suyao'
      }
      await saveConfig()
      message.success('音色已删除')
    }
  })
}

// Clear cache
async function clearCache() {
  try {
    const count = await invoke<number>('clear_tts_cache')
    message.success(`已清理 ${count} 个缓存文件`)
  } catch (err) {
    console.error('Failed to clear cache:', err)
    message.error('清理缓存失败')
  }
}
</script>

<template>
  <div class="tts-settings">
    <ProList>
      <!-- Enable -->
      <ProListItem
        description="启用后 AI 回复将通过 TTS 朗读"
        title="启用 TTS"
      >
        <Switch v-model:checked="enabled" @change="saveConfig" />
      </ProListItem>

      <!-- Base URL -->
      <ProListItem
        v-if="enabled"
        description="IndePTTS2 服务地址"
        title="服务地址"
      >
        <Input
          v-model:value="baseUrl"
          class="w-60"
          placeholder="http://localhost:9880"
          @blur="saveConfig"
        />
      </ProListItem>

      <!-- Volume -->
      <ProListItem
        v-if="enabled"
        description="音量调节"
        title="音量"
      >
        <div class="w-48">
          <Slider
            v-model:value="volume"
            :min="0"
            :max="100"
            :marks="{ 0: '0', 50: '50', 100: '100' }"
            @afterChange="saveConfig"
          />
        </div>
      </ProListItem>

      <!-- Speed -->
      <ProListItem
        v-if="enabled"
        description="语速调节"
        title="语速"
      >
        <div class="w-48">
          <Slider
            v-model:value="speed"
            :min="0.5"
            :max="2.0"
            :step="0.1"
            :marks="{ 0.5: '0.5x', 1: '1x', 2: '2x' }"
            @afterChange="saveConfig"
          />
        </div>
      </ProListItem>

      <!-- Test Connection -->
      <ProListItem
        v-if="enabled"
        description="测试 TTS 服务连接"
        title="连接测试"
      >
        <Button
          :loading="testingConnection"
          type="primary"
          @click="testConnection"
        >
          测试
        </Button>
      </ProListItem>

      <!-- Clear Cache -->
      <ProListItem
        v-if="enabled"
        description="清理 TTS 音频缓存"
        title="清理缓存"
      >
        <Button @click="clearCache">
          清理缓存
        </Button>
      </ProListItem>

      <!-- Voice Management Header -->
      <ProListItem
        v-if="enabled"
        title="音色管理"
      >
        <Button type="primary" @click="openAddVoiceModal">
          添加音色
        </Button>
      </ProListItem>

      <!-- Voice List -->
      <ProListItem
        v-for="voice in voiceList"
        v-if="enabled && voiceList.length > 0"
        :key="voice.id"
        :description="`音色: ${voice.speaker}, 情感: ${voice.emo}`"
        :title="voice.id === defaultVoiceId ? `${voice.id} (默认)` : voice.id"
      >
        <div class="flex gap-2">
          <Button size="small" @click="openEditVoiceModal(voice.id)">
            编辑
          </Button>
          <Button size="small" danger @click="deleteVoice(voice.id)">
            删除
          </Button>
          <Button
            v-if="voice.id !== defaultVoiceId"
            size="small"
            @click="defaultVoiceId = voice.id; saveConfig()"
          >
            设为默认
          </Button>
        </div>
      </ProListItem>
    </ProList>

    <!-- Voice Modal -->
    <Modal
      v-model:open="showVoiceModal"
      :title="editingVoiceId ? '编辑音色' : '添加音色'"
      @ok="saveVoice"
    >
      <div class="space-y-4">
        <div>
          <label class="block mb-1">音色名称 (speaker)</label>
          <Input
            v-model:value="voiceForm.speaker"
            placeholder="如: 苏瑶"
          />
        </div>
        <div>
          <label class="block mb-1">情感 (emo)</label>
          <Input
            v-model:value="voiceForm.emo"
            placeholder="如: neutral, happy, 悲伤, 或 情感参考/愤怒.wav"
          />
        </div>
        <div>
          <label class="block mb-1">情感强度 (weight): {{ voiceForm.weight }}</label>
          <Slider
            v-model:value="voiceForm.weight"
            :min="0"
            :max="1"
            :step="0.1"
          />
        </div>
      </div>
    </Modal>
  </div>
</template>

<style scoped>
.tts-settings {
  padding: 16px;
}
</style>
