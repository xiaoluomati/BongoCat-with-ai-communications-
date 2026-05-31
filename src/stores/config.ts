import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface CharacterConfig {
  id: string
  name: string
  description: string
  avatar: string
  preferred_address: string
  system_prompt: string
  preset_prompt: string
  voice_id?: string
}

export interface LLMConfigState {
  provider: string
  deepseek: { api_key: string; base_url: string; model: string }
  minimax: { api_key: string; base_url: string; model: string }
  'llama.cpp': { api_key: string; base_url: string; model: string }
  stream: boolean
  temperature: number
  max_tokens: number
}

export interface TTSConfigState {
  enabled: boolean
  emotion_auto: boolean
  stream_enabled: boolean
  stream_trigger_threshold: number
  stream_max_buffer: number
  stream_min_chunk: number
  voices: any[]
}

export const useConfigStore = defineStore('config', () => {
  // 角色相关
  const initialized = ref(false)
  const currentCharacterId = ref('')
  const currentCharacter = ref<CharacterConfig | null>(null)

  // LLM 相关
  const llmConfig = ref<LLMConfigState>({
    provider: 'deepseek',
    deepseek: { api_key: '', base_url: 'https://api.deepseek.com', model: '' },
    minimax: { api_key: '', base_url: 'https://api.minimax.chat', model: '' },
    'llama.cpp': { api_key: '', base_url: 'http://localhost', model: '' },
    stream: false,
    temperature: 0.8,
    max_tokens: 500,
  })

  // TTS 相关
  const ttsConfig = ref<TTSConfigState>({
    enabled: false,
    emotion_auto: false,
    stream_enabled: false,
    stream_trigger_threshold: 20,
    stream_max_buffer: 50,
    stream_min_chunk: 5,
    voices: [],
  })

  // 初始化：从后端加载配置
  async function init() {
    await load()
  }

  // 从后端读取完整配置
  async function load() {
    try {
      const cfg = await invoke<any>('load_config')

      // 更新角色
      currentCharacterId.value = cfg.characters?.current || 'cat'
      const charData = await invoke<any>('load_character', { id: currentCharacterId.value })
      currentCharacter.value = charData
      initialized.value = true

      // 更新 LLM
      if (cfg.llm) {
        llmConfig.value = {
          provider: cfg.llm.provider || 'deepseek',
          deepseek: cfg.llm.deepseek || { api_key: '', base_url: 'https://api.deepseek.com', model: '' },
          minimax: cfg.llm.minimax || { api_key: '', base_url: 'https://api.minimax.chat', model: '' },
          'llama.cpp': cfg.llm['llama.cpp'] || { api_key: '', base_url: 'http://localhost', model: '' },
          stream: cfg.llm.stream ?? false,
          temperature: cfg.llm.temperature ?? 0.8,
          max_tokens: cfg.llm.max_tokens ?? 500,
        }
      }

      // 更新 TTS
      if (cfg.tts) {
        ttsConfig.value = {
          enabled: cfg.tts.enabled ?? false,
          emotion_auto: cfg.tts.emotion_auto ?? false,
          stream_enabled: cfg.tts.stream_enabled ?? false,
          stream_trigger_threshold: cfg.tts.stream_trigger_threshold ?? 20,
          stream_max_buffer: cfg.tts.stream_max_buffer ?? 50,
          stream_min_chunk: cfg.tts.stream_min_chunk ?? 5,
          voices: cfg.tts.voices ?? [],
        }
      }
    } catch (err) {
      console.error('[config] load error:', err)
    }
  }

  // 保存 LLM 配置
  async function saveLLM(llm: Partial<LLMConfigState>) {
    const current = await invoke<any>('load_config')
    const newLlm = { ...current.llm, ...llm }
    await invoke('save_config', { config: { ...current, llm: newLlm } })
    llmConfig.value = { ...llmConfig.value, ...llm }
  }

  // 保存 TTS 配置
  async function saveTTS(tts: Partial<TTSConfigState>) {
    const current = await invoke<any>('load_config')
    const newTts = { ...current.tts, ...tts }
    await invoke('save_config', { config: { ...current, tts: newTts } })
    ttsConfig.value = { ...ttsConfig.value, ...tts }
  }

  // 保存角色配置
  async function saveCharacter(char: CharacterConfig) {
    await invoke('save_character', { character: char })
    if (char.id === currentCharacterId.value) {
      currentCharacter.value = char
    }
  }

  // 切换角色
  async function switchCharacter(id: string) {
    await invoke('switch_character', { id })
    currentCharacterId.value = id
    const charData = await invoke<any>('load_character', { id })
    currentCharacter.value = charData
  }

  return {
    initialized, currentCharacterId, currentCharacter,
    llmConfig, ttsConfig,
    init, load,
    saveLLM, saveTTS, saveCharacter, switchCharacter,
  }
})
