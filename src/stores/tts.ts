import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { EMOTION_MAP, DEFAULT_EMOTION } from '@/utils/emotion'

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

interface VoiceConfig {
  speaker: string
  emo: string
  weight: number
  emo_method?: string
  speed?: number
}

interface TTSHistoryItem {
  id: string
  text: string
  voice_id: string
  timestamp: number
}

export const useTTSStore = defineStore('tts', () => {
  // State
  const currentAudio = ref<HTMLAudioElement | null>(null)
  const audioQueue = ref<string[]>([])
  const isPlaying = ref(false)
  const isPaused = ref(false)
  const isEnabled = ref(false)
  const isStreamMode = ref(false)
  const config = ref<TTSConfig | null>(null)
  const currentVoiceId = ref<string | null>(null)
  const currentText = ref('')  // 当前播放的文本
  const isFading = ref(false)

  // TTS buffer for streaming
  const ttsBuffer = ref('')
  
  // Current emotion for TTS
  const currentEmotion = ref<string>(DEFAULT_EMOTION)
  const emotionAutoEnabled = ref(false)

  // Initialize
  async function init() {
    // Load TTS config
    try {
      config.value = await invoke<TTSConfig>('get_tts_config')
      isEnabled.value = config.value.enabled
      isStreamMode.value = config.value.stream_enabled
      currentVoiceId.value = config.value.default_voice_id
      // @ts-ignore - emotion_auto may not exist in old config
      emotionAutoEnabled.value = config.value.emotion_auto ?? false
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

  // Get config value with defaults
  function getThreshold(): number {
    return config.value?.stream_trigger_threshold ?? 20
  }

  function getMaxBuffer(): number {
    return config.value?.stream_max_buffer ?? 50
  }

  function getMinChunk(): number {
    return config.value?.stream_min_chunk ?? 5
  }

  function getFadeDuration(): number {
    return config.value?.fade_duration ?? 200
  }

  // Play next audio in queue with fade
  async function playNext(audioUrl: string, text: string = ''): Promise<void> {
    // Fade out previous
    if (currentAudio.value) {
      await fadeOut(currentAudio.value, getFadeDuration())
      currentAudio.value.pause()
    }

    isPlaying.value = true
    isPaused.value = false
    currentText.value = text

    const audio = new Audio(audioUrl)
    currentAudio.value = audio

    audio.onended = () => {
      currentAudio.value = null
      isPlaying.value = false
      currentText.value = ''
      isFading.value = false

      // Auto play next
      if (audioQueue.value.length > 0) {
        const next = audioQueue.value.shift()
        const nextText = audioQueue.value.length > 0 ? '' : ''
        playNext(next!, nextText)
      }
    }

    audio.onerror = () => {
      console.error('[TTS] Audio play error')
      currentAudio.value = null
      isPlaying.value = false
      currentText.value = ''
      isFading.value = false

      // Continue to next even on error
      if (audioQueue.value.length > 0) {
        const next = audioQueue.value.shift()
        playNext(next!)
      }
    }

    // Set volume
    const targetVolume = (config.value?.volume ?? 80) / 100
    audio.volume = 0

    try {
      await audio.play()
      // Fade in
      await fadeIn(audio, targetVolume, getFadeDuration())
    } catch (err) {
      console.error('[TTS] Play error:', err)
      isPlaying.value = false
      currentAudio.value = null
      isFading.value = false
    }
  }

  // Fade in volume
  async function fadeIn(audio: HTMLAudioElement, targetVolume: number, duration: number): Promise<void> {
    if (duration <= 0) {
      audio.volume = targetVolume
      return
    }

    isFading.value = true
    const steps = 10
    const stepDuration = duration / steps
    const volumeStep = targetVolume / steps

    for (let i = 1; i <= steps; i++) {
      if (!audio) break
      audio.volume = Math.min(volumeStep * i, targetVolume)
      await sleep(stepDuration)
    }

    audio.volume = targetVolume
    isFading.value = false
  }

  // Fade out volume
  async function fadeOut(audio: HTMLAudioElement, duration: number): Promise<void> {
    if (duration <= 0) {
      audio.volume = 0
      return
    }

    isFading.value = true
    const startVolume = audio.volume
    const steps = 10
    const stepDuration = duration / steps
    const volumeStep = startVolume / steps

    for (let i = steps - 1; i >= 0; i--) {
      if (!audio) break
      audio.volume = Math.max(volumeStep * i, 0)
      await sleep(stepDuration)
    }

    audio.volume = 0
    isFading.value = false
  }

  // Utility sleep
  function sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms))
  }

  // Add to queue
  function enqueue(audioUrl: string, text: string = ''): void {
    if (!isPlaying.value && !currentAudio.value) {
      playNext(audioUrl, text)
    } else {
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
    isPaused.value = false
    currentText.value = ''
    isFading.value = false
  }

  // Pause
  function pause(): void {
    if (currentAudio.value && isPlaying.value && !isPaused.value) {
      currentAudio.value.pause()
      isPaused.value = true
    }
  }

  // Resume
  function resume(): void {
    if (currentAudio.value && isPaused.value) {
      currentAudio.value.play()
      isPaused.value = false
    }
  }

  // Skip to next
  function skip(): void {
    if (currentAudio.value) {
      // Trigger onended to play next
      const audio = currentAudio.value
      currentAudio.value = null
      isPlaying.value = false
      audio.dispatchEvent(new Event('ended'))
    }
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
      enqueue(audioUrl, text)
    } catch (err) {
      console.error('[TTS] speak error:', err)
    }
  }

  // Speak with specific emotion
  async function speakWithEmotion(text: string, emotion: string, voiceId?: string): Promise<void> {
    if (!isEnabled.value) {
      console.warn('[TTS] TTS is disabled')
      return
    }

    // Normalize emotion to audio file
    const emotionFile = EMOTION_MAP[emotion] || DEFAULT_EMOTION
    
    try {
      const audioUrl = await invoke<string>('tts_speak_with_emotion', {
        text,
        emotion: emotionFile,
        voiceId: voiceId || null
      })
      enqueue(audioUrl, text)
    } catch (err) {
      console.error('[TTS] speakWithEmotion error:', err)
      // Fallback to regular speak
      speak(text, voiceId).catch(console.error)
    }
  }

  // Set current emotion
  function setEmotion(emotion: string): void {
    currentEmotion.value = EMOTION_MAP[emotion] || DEFAULT_EMOTION
  }

  // Get current emotion
  function getEmotion(): string {
    return currentEmotion.value
  }

  // Enable/disable emotion auto
  function setEmotionAuto(enabled: boolean): void {
    emotionAutoEnabled.value = enabled
  }

  // Stream speak - receives chunk, accumulates, triggers TTS when threshold met
  async function speakStream(chunk: string): Promise<void> {
    if (!isEnabled.value || !isStreamMode.value) return

    // Accumulate chunk to buffer
    ttsBuffer.value += chunk

    // Check if should flush
    const len = ttsBuffer.value.length
    const lastChar = chunk.slice(-1)
    const threshold = getThreshold()
    const maxBuffer = getMaxBuffer()
    const minChunk = getMinChunk()

    const isEndMark = ['。', '！', '？'].includes(lastChar)
    const isLongComma = lastChar === '，' && len >= minChunk
    const isTooLong = len >= maxBuffer

    if (isEndMark || isTooLong || (isLongComma && len >= threshold)) {
      const text = flushBuffer()
      if (text) {
        // Async TTS with current emotion, don't wait
        if (emotionAutoEnabled.value) {
          speakWithEmotion(text, currentEmotion.value).catch(console.error)
        } else {
          speak(text).catch(console.error)
        }
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

  // Reload config
  async function reloadConfig(): Promise<void> {
    try {
      config.value = await invoke<TTSConfig>('get_tts_config')
      isEnabled.value = config.value.enabled
      isStreamMode.value = config.value.stream_enabled
      // @ts-ignore - emotion_auto may not exist in old config
      emotionAutoEnabled.value = config.value.emotion_auto ?? false
    } catch (err) {
      console.error('[TTS] Failed to reload config:', err)
    }
  }

  return {
    // State
    currentAudio,
    audioQueue,
    isPlaying,
    isPaused,
    isEnabled,
    isStreamMode,
    config,
    currentVoiceId,
    currentText,
    isFading,
    ttsBuffer,
    currentEmotion,
    emotionAutoEnabled,

    // Methods
    init,
    speak,
    speakWithEmotion,
    speakStream,
    stop,
    pause,
    resume,
    skip,
    enqueue,
    playNext,
    flushBuffer,
    clearBuffer,
    setEnabled,
    setStreamMode,
    setEmotion,
    getEmotion,
    setEmotionAuto,
    reloadConfig,
    getThreshold,
    getMaxBuffer,
    getMinChunk,
    getFadeDuration
  }
})
