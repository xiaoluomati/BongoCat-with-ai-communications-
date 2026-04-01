<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { Button, Input, InputNumber, message, Modal, Select, Slider, Spin, Switch } from 'ant-design-vue'
import { ref, onMounted, computed } from 'vue'

import ProList from '@/components/pro-list/index.vue'
import ProListItem from '@/components/pro-list-item/index.vue'
import { useTTSStore } from '@/stores/tts'

// TTS Configuration
interface VoiceConfig {
  speaker: string
  emo: string
  weight: number
  emo_method?: string
  speed?: number
}

interface TTSConfig {
  enabled: boolean
  base_url: string
  default_voice_id: string
  volume: number
  speed: number
  voices: Record<string, VoiceConfig>
  stream_enabled: boolean
  stream_trigger_threshold: number
  stream_max_buffer: number
  stream_min_chunk: number
  fade_duration: number
}

const enabled = ref(false)
const baseUrl = ref('http://localhost:9880')
const volume = ref(80)
const streamEnabled = ref(false)
const streamTriggerThreshold = ref(20)
const streamMaxBuffer = ref(50)
const streamMinChunk = ref(5)
const fadeDuration = ref(200)
const voices = ref<Record<string, VoiceConfig>>({})
const defaultVoiceId = ref('suyao')
const testingConnection = ref(false)
const connectionStatus = ref<'unknown' | 'success' | 'failed'>('unknown')

// Server available options
const serverVoices = ref<string[]>([])
const serverEmos = ref<string[]>([])
const fetchingOptions = ref(false)

// Voice Modal
const showVoiceModal = ref(false)
const editingVoiceId = ref('')
const voiceForm = ref<VoiceConfig>({
  speaker: '',
  emo: '',
  weight: 0.8,
  emo_method: '使用情感描述文本控制',
  speed: 1.0
})

// emo_control_method options
const emoMethods = [
  { value: '与音色参考音频相同', label: '与音色参考音频相同' },
  { value: '使用情感参考音频', label: '使用情感参考音频' },
  { value: '使用情感向量控制', label: '使用情感向量控制' },
  { value: '使用情感描述文本控制', label: '使用情感描述文本控制' },
]

const voiceList = computed(() => {
  return Object.entries(voices.value).map(([id, voice]) => ({
    id,
    ...voice
  }))
})

// TTS Store for playback controls
const ttsStore = useTTSStore()

// Load config on mount
onMounted(async () => {
  try {
    const config = await invoke<TTSConfig>('get_tts_config')
    enabled.value = config.enabled
    baseUrl.value = config.base_url
    volume.value = config.volume
    voices.value = config.voices || {}
    defaultVoiceId.value = config.default_voice_id
    streamEnabled.value = config.stream_enabled ?? false
    streamTriggerThreshold.value = config.stream_trigger_threshold ?? 20
    streamMaxBuffer.value = config.stream_max_buffer ?? 50
    streamMinChunk.value = config.stream_min_chunk ?? 5
    fadeDuration.value = config.fade_duration ?? 200
  } catch (err) {
    console.error('Failed to load TTS config:', err)
  }
})

// Fetch available voices and emotions from server
async function fetchServerOptions() {
  fetchingOptions.value = true
  try {
    await saveConfig()
    serverVoices.value = await invoke<string[]>('get_index_tts_voices', { baseUrl: baseUrl.value })
    serverEmos.value = await invoke<string[]>('get_index_tts_emos', { baseUrl: baseUrl.value })
    message.success('已获取服务器音色和情感列表')
  } catch (err) {
    console.error('Failed to fetch options:', err)
    message.error('获取服务器选项失败: ' + err)
  } finally {
    fetchingOptions.value = false
  }
}

// Save config
async function saveConfig() {
  try {
    const config: TTSConfig = {
      enabled: enabled.value,
      base_url: baseUrl.value,
      default_voice_id: defaultVoiceId.value,
      volume: volume.value,
      speed: volume.value,
      voices: voices.value,
      stream_enabled: streamEnabled.value,
      stream_trigger_threshold: streamTriggerThreshold.value,
      stream_max_buffer: streamMaxBuffer.value,
      stream_min_chunk: streamMinChunk.value,
      fade_duration: fadeDuration.value
    }
    await invoke('save_tts_config', { ttsConfig: config })
    // Don't show message here to avoid spam
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
    speaker: serverVoices.value[0] || '',
    emo: serverEmos.value[0] || '',
    weight: 0.8,
    emo_method: '使用情感描述文本控制',
    speed: 1.0
  }
  showVoiceModal.value = true
}

// Open voice modal for edit
function openEditVoiceModal(voiceId: string) {
  editingVoiceId.value = voiceId
  const existing = voices.value[voiceId]
  voiceForm.value = { 
    speaker: existing.speaker,
    emo: existing.emo,
    weight: existing.weight,
    emo_method: existing.emo_method || '使用情感描述文本控制',
    speed: existing.speed || 1.0
  }
  showVoiceModal.value = true
}

// Save voice
async function saveVoice() {
  if (!voiceForm.value.speaker) {
    message.error('请选择音色')
    return
  }
  if (!voiceForm.value.emo) {
    message.error('请选择情感')
    return
  }
  
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
    <Spin :spinning="fetchingOptions" tip="获取服务器选项中...">
      <ProList>
        <!-- Enable -->
        <ProListItem
          description="启用后 AI 回复将通过 TTS 朗读"
          title="启用 TTS"
        >
          <Switch v-model:checked="enabled" @change="saveConfig" />
        </ProListItem>

        <!-- Stream Mode -->
        <ProListItem
          v-if="enabled"
          description="边生成边播放，降低首句延迟"
          title="流式模式"
        >
          <Switch v-model:checked="streamEnabled" @change="saveConfig" />
        </ProListItem>

        <!-- Stream Parameters -->
        <template v-if="enabled && streamEnabled">
          <!-- Trigger Threshold -->
          <ProListItem
            description="累积超过此值时触发 TTS（句子较短时可设低）"
            title="触发阈值"
          >
            <div class="w-48">
              <Slider
                v-model:value="streamTriggerThreshold"
                :min="10"
                :max="50"
                :marks="{ 10: '10', 20: '20', 30: '30', 40: '40', 50: '50' }"
                @afterChange="saveConfig"
              />
            </div>
          </ProListItem>

          <!-- Max Buffer -->
          <ProListItem
            description="超过此值强制触发 TTS（防止单次过长）"
            title="最大缓冲"
          >
            <div class="w-48">
              <Slider
                v-model:value="streamMaxBuffer"
                :min="30"
                :max="100"
                :marks="{ 30: '30', 50: '50', 70: '70', 100: '100' }"
                @afterChange="saveConfig"
              />
            </div>
          </ProListItem>

          <!-- Min Chunk -->
          <ProListItem
            description="必须达到此长度才触发（避免太短的碎片）"
            title="最小触发"
          >
            <div class="w-48">
              <Slider
                v-model:value="streamMinChunk"
                :min="3"
                :max="10"
                :marks="{ 3: '3', 5: '5', 7: '7', 10: '10' }"
                @afterChange="saveConfig"
              />
            </div>
          </ProListItem>

          <!-- Fade Duration -->
          <ProListItem
            description="音频渐变时长（毫秒）"
            title="渐变时长"
          >
            <div class="w-48">
              <Slider
                v-model:value="fadeDuration"
                :min="0"
                :max="500"
                :step="50"
                :marks="{ 0: '0', 200: '200', 400: '400', 500: '500' }"
                @afterChange="saveConfig"
              />
            </div>
          </ProListItem>
        </template>

        <!-- Base URL -->
        <ProListItem
          v-if="enabled"
          description="IndexTTS 服务地址"
          title="服务地址"
        >
          <div class="flex items-center gap-2">
            <Input
              v-model:value="baseUrl"
              class="w-60"
              placeholder="http://localhost:9880"
              @blur="saveConfig"
            />
            <Button
              size="small"
              :loading="fetchingOptions"
              @click="fetchServerOptions"
            >
              获取选项
            </Button>
          </div>
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
          <div class="flex gap-2">
            <Button type="primary" @click="openAddVoiceModal">
              添加音色
            </Button>
            <Button @click="fetchServerOptions" :loading="fetchingOptions">
              刷新选项
            </Button>
          </div>
        </ProListItem>

        <!-- Server Options Info -->
        <ProListItem
          v-if="enabled && serverVoices.length > 0"
          description="从服务器获取的可用选项"
          title="服务器选项"
        >
          <div class="text-sm text-gray-500">
            音色: {{ serverVoices.length }} 个 | 情感: {{ serverEmos.length }} 个
          </div>
        </ProListItem>

        <!-- Voice List -->
        <ProListItem
          v-for="voice in voiceList"
          v-if="enabled && voiceList.length > 0"
          :key="voice.id"
          :description="`音色: ${voice.speaker}, 情感: ${voice.emo}, 方式: ${voice.emo_method || '使用情感描述文本控制'}`"
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
    </Spin>

    <!-- Playback Controls -->
    <div v-if="enabled && ttsStore.isPlaying.value" class="fixed bottom-4 left-1/2 transform -translate-x-1/2 bg-white shadow-lg rounded-lg px-4 py-3 flex items-center gap-3 z-50">
      <span class="text-sm text-gray-600">
        {{ ttsStore.currentText.value?.slice(0, 20) || '播放中...' }}{{ (ttsStore.currentText.value?.length || 0) > 20 ? '...' : '' }}
      </span>
      <Button size="small" @click="ttsStore.isPaused.value ? ttsStore.resume() : ttsStore.pause()">
        {{ ttsStore.isPaused.value ? '继续' : '暂停' }}
      </Button>
      <Button size="small" @click="ttsStore.skip()">
        跳过
      </Button>
      <Button size="small" danger @click="ttsStore.stop()">
        停止
      </Button>
    </div>

    <!-- Voice Modal -->
    <Modal
      v-model:open="showVoiceModal"
      :title="editingVoiceId ? '编辑音色' : '添加音色'"
      @ok="saveVoice"
    >
      <div class="space-y-4">
        <div>
          <label class="block mb-1">音色 (speaker) *</label>
          <Select
            v-if="serverVoices.length > 0"
            v-model:value="voiceForm.speaker"
            class="w-full"
            placeholder="选择音色"
          >
            <Select.Option v-for="v in serverVoices" :key="v" :value="v">
              {{ v }}
            </Select.Option>
          </Select>
          <Input
            v-else
            v-model:value="voiceForm.speaker"
            placeholder="输入音色名称"
          />
        </div>
        <div>
          <label class="block mb-1">情感 (emo) *</label>
          <Select
            v-if="serverEmos.length > 0"
            v-model:value="voiceForm.emo"
            class="w-full"
            placeholder="选择情感"
          >
            <Select.Option v-for="e in serverEmos" :key="e" :value="e">
              {{ e.replace('.wav', '') }}
            </Select.Option>
          </Select>
          <Input
            v-else
            v-model:value="voiceForm.emo"
            placeholder="输入情感标签"
          />
        </div>
        <div>
          <label class="block mb-1">情感控制方式</label>
          <Select
            v-model:value="voiceForm.emo_method"
            class="w-full"
          >
            <Select.Option v-for="m in emoMethods" :key="m.value" :value="m.value">
              {{ m.label }}
            </Select.Option>
          </Select>
        </div>
        <div>
          <label class="block mb-1">情感强度 (weight): {{ voiceForm.weight }}</label>
          <Slider
            v-model:value="voiceForm.weight"
            :min="0"
            :max="1.6"
            :step="0.1"
          />
        </div>
        <div>
          <label class="block mb-1">语速 (speed): {{ voiceForm.speed }}</label>
          <Slider
            v-model:value="voiceForm.speed"
            :min="0.1"
            :max="2.5"
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
