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
  // State
  const currentAudio = ref<HTMLAudioElement | null>(null)
  const audioQueue = ref<string[]>([])  // 播放队列
  const isPlaying = ref(false)
  const isEnabled = ref(false)
  const isStreamMode = ref(false)  // 流式 TTS 开关
  const config = ref<TTSConfig | null>(null)
  const currentVoiceId = ref<string | null>(null)

  // TTS buffer for streaming
  const ttsBuffer = ref('')
  const TTS_THRESHOLD = 20  // 触发阈值（字符数）
  const MAX_BUFFER_LENGTH = 50  // 最大 buffer 长度

  // Initialize
  async function init() {
    // Load TTS config
    try {
      config.value = await invoke<TTSConfig>('get_tts_config')
      isEnabled.value = config.value.enabled
      currentVoiceId.value = config.value.default_voice_id
    } catch (err) {
      console.error('[TTS] Failed to load config:', err)
    }

    // Listen for tts_ready event (non-streaming mode)
    await listen<string>('tts_ready', async (event) => {
      if (!isEnabled.value || isStreamMode.value) return
      const audioUrl = event.payload
      enqueue(audioUrl)
    })
  }

  // Play next audio in queue
  async function playNext(audioUrl: string): Promise<void> {
    isPlaying.value = true
    const audio = new Audio(audioUrl)
    currentAudio.value = audio

    audio.onended = () => {
      currentAudio.value = null
      isPlaying.value = false

      // Auto play next
      if (audioQueue.value.length > 0) {
        const nextUrl = audioQueue.value.shift()
        playNext(nextUrl!)
      }
    }

    audio.onerror = () => {
      console.error('[TTS] Audio play error')
      isPlaying.value = false
      currentAudio.value = null

      // Continue to next even on error
      if (audioQueue.value.length > 0) {
        const nextUrl = audioQueue.value.shift()
        playNext(nextUrl!)
      }
    }

    try {
      await audio.play()
    } catch (err) {
      console.error('[TTS] Play error:', err)
      isPlaying.value = false
      currentAudio.value = null

      // Continue to next even on error
      if (audioQueue.value.length > 0) {
        const nextUrl = audioQueue.value.shift()
        playNext(nextUrl!)
      }
    }
  }

  // Add to queue
  function enqueue(audioUrl: string): void {
    if (!isPlaying.value && !currentAudio.value) {
      // No audio playing, play immediately
      playNext(audioUrl)
    } else {
      // Add to queue
      audioQueue.value.push(audioUrl)
    }
  }

  // Stop and clear queue
  function stop(): void {
    if (currentAudio.value) {
      currentAudio.value.pause()
      currentAudio.value = null
    }
    audioQueue.value = []
    isPlaying.value = false
  }

  // Manual speak (for testing, non-streaming mode)
  async function speak(text: string, voiceId?: string): Promise<void> {
    if (!isEnabled.value) {
      console.warn('[TTS] TTS is disabled')
      return
    }

    try {
      const audioUrl = await invoke<string>('tts_speak', {
        text,
        voiceId: voiceId || null
      })
      enqueue(audioUrl)
    } catch (err) {
      console.error('[TTS] speak error:', err)
    }
  }

  // Stream speak - receives chunk, accumulates, triggers TTS when threshold met
  async function speakStream(chunk: string): Promise<void> {
    if (!isEnabled.value) return

    // Accumulate chunk to buffer
    ttsBuffer.value += chunk

    // Check if should flush
    const len = ttsBuffer.value.length
    const lastChar = chunk.slice(-1)
    const isEndMark = ['。', '！', '？'].includes(lastChar)
    const isLongComma = lastChar === '，' && len >= 10
    const isTooLong = len >= MAX_BUFFER_LENGTH

    if (isEndMark || isTooLong || (isLongComma && len >= TTS_THRESHOLD)) {
      const text = flushBuffer()
      if (text) {
        // Async TTS, don't wait
        speak(text).catch(console.error)
      }
    }
  }

  // Flush buffer and return text
  function flushBuffer(): string {
    const text = ttsBuffer.value
    ttsBuffer.value = ''
    return text
  }

  // Clear buffer without triggering
  function clearBuffer(): void {
    ttsBuffer.value = ''
  }

  // Update config
  function setEnabled(enabled: boolean): void {
    isEnabled.value = enabled
    if (!enabled) {
      stop()
      clearBuffer()
    }
  }

  // Set stream mode
  function setStreamMode(enabled: boolean): void {
    isStreamMode.value = enabled
    if (!enabled) {
      stop()
      clearBuffer()
    }
  }

  // Set current voice
  function setCurrentVoice(voiceId: string): void {
    currentVoiceId.value = voiceId
  }

  return {
    // State
    currentAudio,
    audioQueue,
    isPlaying,
    isEnabled,
    isStreamMode,
    config,
    currentVoiceId,
    ttsBuffer,

    // Methods
    init,
    speak,
    speakStream,
    stop,
    enqueue,
    playNext,
    flushBuffer,
    clearBuffer,
    setEnabled,
    setStreamMode,
    setCurrentVoice
  }
})
