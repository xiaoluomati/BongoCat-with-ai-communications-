import { convertFileSrc } from '@tauri-apps/api/core'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification'
import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

export const useDrinkWaterStore = defineStore('drinkWater', () => {
  const enabled = ref(false)
  const interval = ref(30) // minutes
  const notificationEnabled = ref(true)
  const soundEnabled = ref(true)
  const soundPaths = ref<string[]>([])
  const volume = ref(80)

  const timer = ref<NodeJS.Timeout | null>(null)
  const audioElement = ref<HTMLAudioElement | null>(null)
  const isPlaying = ref(false)

  // Initialize Audio
  const initAudio = () => {
    if (!audioElement.value) {
      audioElement.value = new Audio()
      audioElement.value.crossOrigin = 'anonymous'

      audioElement.value.addEventListener('ended', () => {
        isPlaying.value = false
      })

      audioElement.value.addEventListener('error', (e) => {
        console.error('Audio playback error:', e)
        isPlaying.value = false
      })
    }
    audioElement.value.volume = volume.value / 100
  }

  // Set Volume
  const setVolume = (vol: number) => {
    volume.value = vol
    if (audioElement.value) {
      audioElement.value.volume = vol / 100
    }
  }

  // Play Sound
  const playSound = async () => {
    if (!soundEnabled.value || soundPaths.value.length === 0) return

    initAudio()
    if (audioElement.value) {
      try {
        // Randomly select one sound file
        const randomIndex = Math.floor(Math.random() * soundPaths.value.length)
        const selectedSound = soundPaths.value[randomIndex]

        const audioSrc = convertFileSrc(selectedSound)
        audioElement.value.src = audioSrc
        await audioElement.value.play()
        isPlaying.value = true
      } catch (error) {
        console.error('Failed to play sound:', error)
        isPlaying.value = false
      }
    }
  }

  // Send Notification
  const sendAlert = async () => {
    if (notificationEnabled.value) {
      let permissionGranted = await isPermissionGranted()
      if (!permissionGranted) {
        const permission = await requestPermission()
        permissionGranted = permission === 'granted'
      }

      if (permissionGranted) {
        sendNotification({
          title: '喝水时间',
          body: '该喝水啦！保持水分充足哦~ 💧',
        })
      }
    }

    if (soundEnabled.value) {
      playSound()
    }
  }

  // Start Timer
  const startTimer = () => {
    stopTimer()
    const appWindow = getCurrentWebviewWindow()
    // only run timer in main window
    if (appWindow.label !== 'main') return

    if (enabled.value && interval.value > 0) {
      // Convert minutes to milliseconds
      const ms = interval.value * 60 * 1000
      timer.value = setInterval(() => {
        sendAlert()
      }, ms)
    }
  }

  // Stop Timer
  const stopTimer = () => {
    if (timer.value) {
      clearInterval(timer.value)
      timer.value = null
    }
  }

  // Watch for changes to restart timer
  watch([enabled, interval], () => {
    if (enabled.value) {
      startTimer()
    } else {
      stopTimer()
    }
  })

  // Initialize
  const init = () => {
    if (enabled.value) {
      startTimer()
    }
  }

  return {
    enabled,
    interval,
    notificationEnabled,
    soundEnabled,
    soundPaths,
    volume,
    isPlaying,
    init,
    setVolume,
    playSound,
    sendAlert,
  }
}, {
  tauri: {
    filterKeys: ['enabled', 'interval', 'notificationEnabled', 'soundEnabled', 'soundPaths', 'volume'],
    filterKeysStrategy: 'pick',
  },
})
