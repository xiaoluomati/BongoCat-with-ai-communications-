import { convertFileSrc } from '@tauri-apps/api/core'
import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface Song {
  id: string
  title: string
  // artist: string
  url: string
  duration?: number
}

export interface Playlist {
  id: string
  name: string
  songs: Song[]
  createdAt: number
}

export const useMusicStore = defineStore('music', () => {
  const currentSong = ref<Song | null>(null)
  const isPlaying = ref(false)
  const currentTime = ref(0)
  const volume = ref(80)
  const repeatMode = ref<'none' | 'single' | 'all'>('none')
  const shuffleMode = ref(false)

  // 音频元素（不需要持久化）
  const audioElement = ref<HTMLAudioElement | null>(null)

  // 歌单数据（会自动持久化）
  const playlists = ref<Playlist[]>([
    {
      id: 'default',
      name: '默认歌单',
      songs: [],
      createdAt: Date.now(),
    },
  ])

  const currentPlaylistId = ref<string>('default')

  // 当前播放列表 - 使用 ref 而不是 computed
  const currentPlaylist = ref<Playlist | null>(null)

  // 初始化当前播放列表
  const initCurrentPlaylist = () => {
    const playlist = playlists.value.find(p => p.id === currentPlaylistId.value)
    currentPlaylist.value = playlist || playlists.value[0] || null
  }

  // 设置当前播放列表
  const setCurrentPlaylist = (playlistId: string) => {
    const playlist = playlists.value.find(p => p.id === playlistId)
    if (playlist) {
      currentPlaylistId.value = playlistId
      currentPlaylist.value = playlist
    }
  }

  // 所有方法保持不变...
  const initAudio = () => {
    if (!audioElement.value) {
      audioElement.value = new Audio()
      audioElement.value.crossOrigin = 'anonymous'

      audioElement.value.addEventListener('loadedmetadata', () => {
        if (currentSong.value && audioElement.value) {
          currentSong.value.duration = audioElement.value.duration
        }
      })

      audioElement.value.addEventListener('timeupdate', () => {
        if (audioElement.value && !audioElement.value.paused) {
          currentTime.value = audioElement.value.currentTime
        }
      })

      audioElement.value.addEventListener('ended', () => {
        handleSongEnd()
      })

      audioElement.value.addEventListener('error', (e) => {
        console.error('音频播放错误:', e)
        isPlaying.value = false
      })

      audioElement.value.addEventListener('play', () => {
        isPlaying.value = true
      })

      audioElement.value.addEventListener('pause', () => {
        isPlaying.value = false
      })

      audioElement.value.volume = volume.value / 100
    }
  }

  const play = async () => {
    if (!audioElement.value || !currentSong.value) return

    try {
      await audioElement.value.play()
      isPlaying.value = true
    } catch (error) {
      console.error('播放失败:', error)
      isPlaying.value = false
    }
  }

  const pause = () => {
    if (audioElement.value) {
      audioElement.value.pause()
      isPlaying.value = false
    }
  }

  const togglePlay = () => {
    if (isPlaying.value) {
      pause()
    } else {
      play()
    }
  }

  const playSong = async (song: Song, playlistId: string) => {
    initAudio()
    currentSong.value = song
    currentTime.value = 0

    if (audioElement.value) {
      try {
        const audioSrc = convertFileSrc(song.url)
        audioElement.value.src = audioSrc
        audioElement.value.currentTime = 0
        audioElement.value.load()
        setCurrentPlaylist(playlistId)
        setTimeout(async () => {
          try {
            await play()
          } catch (error) {
            console.error('延迟播放失败:', error)
          }
        }, 100)
      } catch (error) {
        console.error('设置音频源失败:', error)
      }
    }
  }

  const handleSongEnd = () => {
    if (repeatMode.value === 'single') {
      if (audioElement.value) {
        audioElement.value.currentTime = 0
        play()
      }
    } else {
      nextSong()
    }
  }

  const nextSong = () => {
    if (!currentPlaylist.value || currentPlaylist.value.songs.length === 0) return

    const currentIndex = currentPlaylist.value.songs.findIndex(s => s.id === currentSong.value?.id)
    let nextIndex = currentIndex + 1

    if (shuffleMode.value) {
      do {
        nextIndex = Math.floor(Math.random() * currentPlaylist.value.songs.length)
      } while (nextIndex === currentIndex && currentPlaylist.value.songs.length > 1)
    } else if (nextIndex >= currentPlaylist.value.songs.length) {
      if (repeatMode.value === 'all') {
        nextIndex = 0
      } else {
        pause()
        return
      }
    }

    if (currentPlaylist.value.songs[nextIndex]) {
      playSong(currentPlaylist.value.songs[nextIndex], currentPlaylist.value.id)
    }
  }

  const prevSong = () => {
    if (!currentPlaylist.value || currentPlaylist.value.songs.length === 0) return

    const currentIndex = currentPlaylist.value.songs.findIndex(s => s.id === currentSong.value?.id)
    let prevIndex = currentIndex - 1

    if (shuffleMode.value) {
      do {
        prevIndex = Math.floor(Math.random() * currentPlaylist.value.songs.length)
      } while (prevIndex === currentIndex && currentPlaylist.value.songs.length > 1)
    } else if (prevIndex < 0) {
      if (repeatMode.value === 'all') {
        prevIndex = currentPlaylist.value.songs.length - 1
      } else {
        return
      }
    }

    if (currentPlaylist.value.songs[prevIndex]) {
      playSong(currentPlaylist.value.songs[prevIndex], currentPlaylist.value.id)
    }
  }

  const createPlaylist = (name: string) => {
    const newPlaylist: Playlist = {
      id: `playlist_${Date.now()}`,
      name,
      songs: [],
      createdAt: Date.now(),
    }
    playlists.value.push(newPlaylist)
    return newPlaylist
  }

  const deletePlaylist = (id: string) => {
    const index = playlists.value.findIndex(p => p.id === id)
    if (index > -1) {
      playlists.value.splice(index, 1)

      // 如果删除的是当前播放列表，切换到默认歌单
      if (currentPlaylistId.value === id) {
        const firstPlaylist = playlists.value[0]
        if (firstPlaylist) {
          setCurrentPlaylist(firstPlaylist.id)
        } else {
          currentPlaylistId.value = 'default'
          currentPlaylist.value = null
        }
      }
    }
  }

  const addSongToPlaylist = (playlistId: string, song: Song) => {
    const playlist = playlists.value.find(p => p.id === playlistId)
    if (playlist) {
      const exists = playlist.songs.some(s => s.url === song.url)
      if (!exists) {
        playlist.songs.push(song)

        // 如果添加到当前播放列表，更新当前播放列表引用
        if (playlistId === currentPlaylistId.value) {
          currentPlaylist.value = playlist
        }

        return true
      }
    }
    return false
  }

  const removeSongFromPlaylist = (playlistId: string, songId: string) => {
    const playlist = playlists.value.find(p => p.id === playlistId)
    if (playlist) {
      const index = playlist.songs.findIndex(s => s.id === songId)
      if (index > -1) {
        playlist.songs.splice(index, 1)

        // 如果从当前播放列表删除，更新当前播放列表引用
        if (playlistId === currentPlaylistId.value) {
          currentPlaylist.value = playlist
        }

        return true
      }
    }
    return false
  }

  const setCurrentTime = (time: number) => {
    if (audioElement.value && currentSong.value) {
      audioElement.value.currentTime = time
      currentTime.value = time
    }
  }

  const setVolume = (vol: number) => {
    volume.value = vol
    if (audioElement.value) {
      audioElement.value.volume = vol / 100
    }
  }

  const cleanup = () => {
    if (audioElement.value) {
      audioElement.value.pause()
      audioElement.value.src = ''
      audioElement.value = null
    }
  }

  // 初始化方法
  const init = () => {
    initCurrentPlaylist()
    initAudio()
  }

  return {
    currentSong,
    isPlaying,
    currentTime,
    volume,
    playlists,
    currentPlaylist,
    currentPlaylistId,
    repeatMode,
    shuffleMode,
    audioElement,
    init,
    initAudio,
    initCurrentPlaylist,
    play,
    pause,
    togglePlay,
    playSong,
    nextSong,
    prevSong,
    setCurrentTime,
    setVolume,
    createPlaylist,
    deletePlaylist,
    addSongToPlaylist,
    removeSongFromPlaylist,
    setCurrentPlaylist,
    cleanup,
  }
}, {
  // 使用与模型管理相同的持久化配置
  tauri: {
    filterKeys: ['volume', 'repeatMode', 'shuffleMode', 'playlists', 'currentPlaylistId'],
    filterKeysStrategy: 'pick',
  },
})
