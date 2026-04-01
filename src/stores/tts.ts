import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

interface TTSConfig {
  enabled: boolean
  base_url: string
  default_voice_id: string
  volume: number
  speed: number
  voices: Record<string, VoiceConfig>
}

interface VoiceConfig {
  speaker: string
  emo: string
  weight: number
  emo_method?: string
  speed?: number
}

export const useTTSStore = defineStore('tts', () => {
  const currentAudio = ref<HTMLAudioElement | null>(null)
  const isPlaying = ref(false)
  const isEnabled = ref(false)
  const config = ref<TTSConfig | null>(null)

  // Listen for TTS ready event from backend
  async function init() {
    // Load TTS config
    try {
      config.value = await invoke<TTSConfig>('get_tts_config')
      isEnabled.value = config.value.enabled
    } catch (err) {
      console.error('Failed to load TTS config:', err)
    }

    // Listen for tts_ready event
    await listen<string>('tts_ready', async (event) => {
      if (!isEnabled.value) return
      
      const audioUrl = event.payload
      await playAudio(audioUrl)
    })
  }

  async function playAudio(audioUrl: string) {
    try {
      // Stop current playback
      if (currentAudio.value) {
        currentAudio.value.pause()
        currentAudio.value = null
      }

      // Create new audio and play
      const audio = new Audio(audioUrl)
      currentAudio.value = audio
      isPlaying.value = true

      audio.onended = () => {
        isPlaying.value = false
        currentAudio.value = null
      }

      audio.onerror = (err) => {
        console.error('Audio playback error:', err)
        isPlaying.value = false
        currentAudio.value = null
      }

      await audio.play()
    } catch (err) {
      console.error('TTS play error:', err)
      isPlaying.value = false
    }
  }

  function stop() {
    if (currentAudio.value) {
      currentAudio.value.pause()
      currentAudio.value = null
      isPlaying.value = false
    }
  }

  // Manual speak (for testing)
  async function speak(text: string, voiceId?: string) {
    if (!isEnabled.value) {
      console.warn('TTS is disabled')
      return
    }

    try {
      const audioUrl = await invoke<string>('tts_speak', {
        text,
        voiceId: voiceId || null
      })
      await playAudio(audioUrl)
    } catch (err) {
      console.error('TTS error:', err)
    }
  }

  // Update enabled state from config
  function setEnabled(enabled: boolean) {
    isEnabled.value = enabled
  }

  return {
    currentAudio,
    isPlaying,
    isEnabled,
    config,
    init,
    speak,
    stop,
    setEnabled
  }
})
