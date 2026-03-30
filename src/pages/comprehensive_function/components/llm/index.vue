<script setup lang="ts">
import { CheckCircleOutlined, CloseCircleOutlined, LinkOutlined } from '@ant-design/icons-vue'
import { Button, Input, InputNumber, message, Select, Slider, Switch } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import { ref, onMounted, watch } from 'vue'

import ProList from '@/components/pro-list/index.vue'
import ProListItem from '@/components/pro-list-item/index.vue'

// State
const enabled = ref(false)
const provider = ref('deepseek')
const apiKey = ref('')
const baseHost = ref('http://localhost')
const basePort = ref(11434)
const model = ref('deepseek-chat')
const temperature = ref(0.8)
const maxTokens = ref(500)
const stream = ref(false)
const testingConnection = ref(false)
const connectionStatus = ref<'unknown' | 'success' | 'failed'>('unknown')

const providers = [
  { value: 'ollama', label: 'Ollama (本地)' },
  { value: 'deepseek', label: 'DeepSeek' },
  { value: 'minimax', label: 'MiniMax' },
]

const defaultModels: Record<string, string> = {
  ollama: 'llama2',
  deepseek: 'deepseek-chat',
  minimax: 'MiniMax-M2.7',
}

const modelOptions: Record<string, { value: string; label: string }[]> = {
  ollama: [
    { value: 'llama2', label: 'llama2' },
    { value: 'llama3', label: 'llama3' },
    { value: 'qwen2.5', label: 'qwen2.5' },
    { value: 'deepseek-coder', label: 'deepseek-coder' },
  ],
  deepseek: [
    { value: 'deepseek-chat', label: 'deepseek-chat' },
    { value: 'deepseek-reasoner', label: 'deepseek-reasoner' },
  ],
  minimax: [
    { value: 'MiniMax-M2.7', label: 'MiniMax-M2.7 (推荐)' },
    { value: 'MiniMax-M2.7-highspeed', label: 'MiniMax-M2.7-highspeed (极速版)' },
    { value: 'MiniMax-M2.5', label: 'MiniMax-M2.5' },
    { value: 'MiniMax-M2.5-highspeed', label: 'MiniMax-M2.5-highspeed (极速版)' },
    { value: 'MiniMax-M2.1', label: 'MiniMax-M2.1 (编程)' },
    { value: 'MiniMax-M2.1-highspeed', label: 'MiniMax-M2.1-highspeed (极速版)' },
    { value: 'MiniMax-M2', label: 'MiniMax-M2 (编码/Agent)' },
  ],
}

const currentModels = () => modelOptions[provider.value] || modelOptions.deepseek

// Load config on mount
onMounted(async () => {
  try {
    const config = await invoke<any>('load_config')
    enabled.value = config.chat?.enabled ?? false
    provider.value = config.llm?.provider || 'deepseek'
    temperature.value = config.llm?.temperature ?? 0.8
    maxTokens.value = config.llm?.max_tokens ?? 500
    stream.value = config.llm?.stream ?? false
    
    // 加载提供商特定配置
    await loadProviderConfig()
  } catch (err) {
    console.error('Failed to load config:', err)
  }
})

// 加载当前提供商的配置
async function loadProviderConfig() {
  try {
    const config = await invoke<any>('load_config')
    const providerConfig = config.llm?.[provider.value]
    
    if (providerConfig) {
      apiKey.value = providerConfig.api_key || ''
      model.value = providerConfig.model || defaultModels[provider.value]
      // 解析 base_url 为 host 和 port
      const fullUrl = providerConfig.base_url || getDefaultBaseUrl() + ':' + getDefaultPort()
      const match = fullUrl.match(/^(https?:\/\/[^:]+)(:(\d+))?$/)
      if (match) {
        baseHost.value = match[1]
        basePort.value = match[3] ? parseInt(match[3]) : getDefaultPort()
      } else {
        baseHost.value = getDefaultBaseUrl()
        basePort.value = getDefaultPort()
      }
    } else {
      // 使用默认值
      apiKey.value = ''
      model.value = defaultModels[provider.value]
      baseHost.value = getDefaultBaseUrl()
      basePort.value = getDefaultPort()
    }
  } catch (err) {
    console.error('Failed to load provider config:', err)
  }
}

// 获取默认 base_url
function getDefaultBaseUrl(): string {
  return provider.value === 'ollama' ? 'http://localhost' : ''
}

// 获取默认端口
function getDefaultPort(): number {
  return provider.value === 'ollama' ? 11434 : 0
}

// 监听提供商变化
watch(provider, async () => {
  await loadProviderConfig()
})

// Save config
async function saveConfig() {
  try {
    const config = await invoke<any>('load_config')
    
    config.chat = config.chat || {}
    config.chat.enabled = enabled.value
    
    config.llm = config.llm || {}
    config.llm.provider = provider.value
    config.llm.temperature = temperature.value
    config.llm.max_tokens = maxTokens.value
    config.llm.stream = stream.value
    
    // 保存提供商特定配置
    config.llm[provider.value] = config.llm[provider.value] || {}
    config.llm[provider.value].api_key = apiKey.value
    config.llm[provider.value].model = model.value
    // 拼接 host 和 port 为完整的 base_url
    config.llm[provider.value].base_url = `${baseHost.value}:${basePort.value}`
    
    await invoke('save_config', { config })
    message.success('配置已保存')
  } catch (err) {
    console.error('Failed to save config:', err)
    message.error('保存配置失败')
  }
}

// Test connection
async function testConnection() {
  testingConnection.value = true
  connectionStatus.value = 'unknown'
  
  try {
    // Save config first
    await saveConfig()
    
    // Test connection
    const isAvailable = await invoke<boolean>('check_llm_available')
    
    if (isAvailable) {
      connectionStatus.value = 'success'
      message.success('连接成功！')
    } else {
      connectionStatus.value = 'failed'
      message.error('连接失败，请检查配置')
    }
  } catch (err) {
    console.error('Connection test failed:', err)
    connectionStatus.value = 'failed'
    message.error('连接失败: ' + err)
  } finally {
    testingConnection.value = false
  }
}

// Watch provider change to update model list
function onProviderChange() {
  model.value = defaultModels[provider.value]
  baseHost.value = getDefaultBaseUrl()
  basePort.value = getDefaultPort()
}
</script>

<template>
  <div class="llm-settings">
    <ProList>
      <!-- Enable -->
      <ProListItem
        description="启用后可通过右键菜单打开聊天窗口"
        title="启用 AI 对话"
      >
        <Switch v-model:checked="enabled" />
      </ProListItem>

      <!-- Provider -->
      <ProListItem
        v-if="enabled"
        description="选择 AI 服务提供商"
        title="服务提供商"
      >
        <Select
          v-model:value="provider"
          class="w-40"
          :options="providers"
          @change="onProviderChange"
        />
      </ProListItem>

      <!-- API Key -->
      <ProListItem
        v-if="enabled && provider !== 'ollama'"
        description="请输入 API Key"
        title="API Key"
      >
        <Input
          v-model:value="apiKey"
          class="w-60"
          placeholder="输入 API Key"
          type="password"
        />
      </ProListItem>

      <!-- Base URL (仅 Ollama 显示) -->
      <ProListItem
        v-if="enabled && provider === 'ollama'"
        description="Ollama 服务地址"
        title="服务地址"
      >
        <div class="flex items-center gap-2">
          <Input
            v-model:value="baseHost"
            class="w-40"
            placeholder="http://localhost"
          />
          <span class="text-gray-500">:</span>
          <InputNumber
            v-model:value="basePort"
            class="w-24"
            :min="1"
            :max="65535"
            :step="1"
            placeholder="11434"
          />
        </div>
      </ProListItem>

      <!-- Model -->
      <ProListItem
        v-if="enabled"
        description="选择对话模型"
        title="模型"
      >
        <Select
          v-model:value="model"
          class="w-48"
          :options="currentModels()"
        />
      </ProListItem>

      <!-- Temperature -->
      <ProListItem
        v-if="enabled"
        description="控制输出的随机性，值越高越随机"
        title="Temperature"
      >
        <div class="flex items-center gap-3">
          <Slider
            v-model:value="temperature"
            class="w-32"
            :max="2"
            :min="0"
            :step="0.1"
          />
          <span class="w-12 text-right text-xs">{{ temperature }}</span>
        </div>
      </ProListItem>

      <!-- Max Tokens -->
      <ProListItem
        v-if="enabled"
        description="限制单次回复的最大 Token 数"
        title="Max Tokens"
      >
        <InputNumber
          v-model:value="maxTokens"
          :max="4000"
          :min="100"
          :step="100"
        />
      </ProListItem>

      <!-- Stream -->
      <ProListItem
        v-if="enabled"
        description="开启后消息会逐字逐句显示，更流畅的体验"
        title="流式输出"
      >
        <Switch v-model:checked="stream" />
      </ProListItem>

      <!-- Test Connection -->
      <ProListItem
        v-if="enabled"
        title="连接测试"
      >
        <div class="flex items-center gap-2">
          <Button
            :loading="testingConnection"
            @click="testConnection"
          >
            <template #icon>
              <LinkOutlined />
            </template>
            测试连接
          </Button>
          
          <CheckCircleOutlined 
            v-if="connectionStatus === 'success'" 
            class="text-green-500 text-lg"
          />
          <CloseCircleOutlined 
            v-if="connectionStatus === 'failed'" 
            class="text-red-500 text-lg"
          />
        </div>
      </ProListItem>

      <!-- Save Button -->
      <ProListItem
        v-if="enabled"
        title=""
      >
        <Button
          type="primary"
          @click="saveConfig"
        >
          保存配置
        </Button>
      </ProListItem>
    </ProList>
  </div>
</template>

<style scoped>
.llm-settings {
  user-select: none;
}
</style>
