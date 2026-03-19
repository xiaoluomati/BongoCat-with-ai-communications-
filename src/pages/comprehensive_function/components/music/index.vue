<script setup lang="ts">
import type { Playlist, Song } from '@/stores/music'

import {
  DeleteOutlined,
  EditOutlined,
  FolderOpenOutlined,
  PauseCircleOutlined,
  PlayCircleOutlined,
  SoundOutlined,
  StepBackwardOutlined,
  StepForwardOutlined,
} from '@ant-design/icons-vue'
import { open } from '@tauri-apps/plugin-dialog'
import { error, info } from '@tauri-apps/plugin-log'
import { Button, Collapse, Input, List, message, Modal, Select, Slider } from 'ant-design-vue'
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'

import ProList from '@/components/pro-list/index.vue'
import ProListItem from '@/components/pro-list-item/index.vue'
import { useMusicStore } from '@/stores/music'

const musicStore = useMusicStore()

const showCreatePlaylistModal = ref(false)
const showAddSongModal = ref(false)
const newPlaylistName = ref('')
const selectedFiles = ref<string[]>([])
const selectedPlaylistId = ref<string>('default')

// 本地状态用于进度条拖拽
const isDraggingProgress = ref(false)
const isDraggingVolume = ref(false)
const localCurrentTime = ref(0)
const localVolume = ref(80)

const currentDuration = computed(() => {
  return formatTime(isDraggingProgress.value ? localCurrentTime.value : musicStore.currentTime)
})

const totalDuration = computed(() => {
  if (musicStore.currentSong?.duration) {
    return formatTime(musicStore.currentSong.duration)
  }
  return '00:00'
})

const playlistOptions = computed(() => {
  return musicStore.playlists.map(playlist => ({
    value: playlist.id,
    label: playlist.name,
  }))
})

const repeatModeText = computed(() => {
  switch (musicStore.repeatMode) {
    case 'single': return '单曲循环'
    case 'all': return '列表循环'
    default: return '顺序播放'
  }
})

const repeatModeIcon = computed(() => {
  switch (musicStore.repeatMode) {
    case 'single': return 'i-solar-repeat-one-bold'
    case 'all': return 'i-solar-repeat-bold'
    default: return 'i-solar-music-notes-bold'
  }
})

// 新增：折叠状态管理
const activeCollapseKeys = ref<string[]>([])

// 展开当前播放的歌单
function expandCurrentPlaylist() {
  if (musicStore.currentPlaylistId && !activeCollapseKeys.value.includes(musicStore.currentPlaylistId)) {
    activeCollapseKeys.value.push(musicStore.currentPlaylistId)
  }
}

// 格式化总时长
function formatTotalDuration(songs: Song[]): string {
  const totalSeconds = songs.reduce((total, song) => {
    return total + (song.duration || 0)
  }, 0)

  if (totalSeconds === 0) return '00:00'

  if (totalSeconds < 3600) {
    // 小于1小时，显示 mm:ss 格式
    return formatTime(totalSeconds)
  } else {
    // 大于1小时，显示 h小时m分钟 格式
    const hours = Math.floor(totalSeconds / 3600)
    const minutes = Math.floor((totalSeconds % 3600) / 60)
    return `${hours}小时${minutes}分钟`
  }
}

// 监听音频播放进度，只在非拖拽状态下同步
watch(() => musicStore.currentTime, (newTime) => {
  if (!isDraggingProgress.value) {
    localCurrentTime.value = newTime
  }
})

// 监听音量变化，只在非拖拽状态下同步
watch(() => musicStore.volume, (newVolume) => {
  if (!isDraggingVolume.value) {
    localVolume.value = newVolume
  }
})
onMounted(async () => {
  musicStore.init() // 初始化 store，包括当前播放列表
  localCurrentTime.value = musicStore.currentTime
  localVolume.value = musicStore.volume

  expandCurrentPlaylist()
})

onUnmounted(() => {
  musicStore.cleanup()
})

function formatTime(seconds: number): string {
  if (!seconds || Number.isNaN(seconds)) return '00:00'
  const mins = Math.floor(seconds / 60)
  const secs = Math.floor(seconds % 60)
  return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`
}

// 进度条拖拽处理 - 修正类型
function onProgressSliderChange(value: number | number[]) {
  const timeValue = Array.isArray(value) ? value[0] : value
  localCurrentTime.value = timeValue
  if (!isDraggingProgress.value) {
    musicStore.setCurrentTime(timeValue)
  }
}

function onProgressSliderAfterChange(value: number | number[]) {
  const timeValue = Array.isArray(value) ? value[0] : value
  musicStore.setCurrentTime(timeValue)
  isDraggingProgress.value = false
}

function onProgressSliderStart() {
  isDraggingProgress.value = true
}

// 音量拖拽处理
function onVolumeSliderChange(value: number | number[]) {
  const volumeValue = Array.isArray(value) ? value[0] : value
  localVolume.value = volumeValue
  musicStore.setVolume(volumeValue)
}

function onVolumeSliderStart() {
  isDraggingVolume.value = true
}

function onVolumeSliderAfterChange() {
  isDraggingVolume.value = false
}

function handleCreatePlaylist() {
  if (newPlaylistName.value.trim()) {
    const newPlaylist = musicStore.createPlaylist(newPlaylistName.value.trim())
    newPlaylistName.value = ''
    showCreatePlaylistModal.value = false
    message.success('歌单创建成功')

    // 自动选择新创建的歌单
    selectedPlaylistId.value = newPlaylist.id
  }
}

async function handleSelectLocalFiles() {
  try {
    const files = await open({
      multiple: true,
      filters: [
        {
          name: '音频文件',
          extensions: ['mp3', 'wav', 'flac', 'aac', 'm4a', 'ogg', 'wma'],
        },
      ],
    })

    if (files && Array.isArray(files)) {
      selectedFiles.value = files
    } else if (files) {
      selectedFiles.value = [files]
    }
  } catch (error) {
    console.error('选择文件失败:', error)
    message.error('选择文件失败')
  }
}

function handleAddSelectedSongs() {
  if (selectedFiles.value.length === 0) {
    message.warning('请先选择音乐文件')
    return
  }

  if (!selectedPlaylistId.value) {
    message.warning('请选择要添加到的歌单')
    return
  }

  const targetPlaylist = musicStore.playlists.find(p => p.id === selectedPlaylistId.value)
  if (!targetPlaylist) {
    message.error('找不到指定的歌单')
    return
  }

  let addedCount = 0
  selectedFiles.value.forEach((filePath) => {
    const fileName = filePath.split(/[/\\]/).pop() || ''
    const songName = fileName.replace(/\.[^/.]+$/, '') // 移除扩展名

    const song: Song = {
      id: `song_${Date.now()}_${Math.random()}`,
      title: songName,
      // artist: '未知艺术家',
      url: filePath,
    }

    // 添加歌曲到歌单
    if (musicStore.addSongToPlaylist(selectedPlaylistId.value, song)) {
      addedCount++
    }
  })

  selectedFiles.value = []
  showAddSongModal.value = false

  if (addedCount > 0) {
    message.success(`成功添加 ${addedCount} 首歌曲到 "${targetPlaylist.name}"`)
  } else {
    message.info('所选歌曲已存在于该歌单中')
  }
}

function handlePlayPlaylist(playlist: Playlist) {
  musicStore.setCurrentPlaylist(playlist.id) // 使用方法设置当前播放列表
  if (playlist.songs.length > 0) {
    musicStore.playSong(playlist.songs[0], playlist.id)
  }
}
// 使用 Modal 确认删除
function confirmDeletePlaylist(playlist: Playlist) {
  info(`确认删除歌单:${playlist.name}`)

  Modal.confirm({
    title: '删除歌单',
    content: `确定要删除歌单 "${playlist.name}" 吗？此操作无法撤销。`,
    okText: '确定删除',
    okType: 'danger',
    cancelText: '取消',
    onOk() {
      handleDeletePlaylist(playlist.id)
    },
    onCancel() {
      info('取消删除')
    },
  })
}
// 添加调试版本的删除函数
function handleDeletePlaylist(id: string) {
  info(`开始删除歌单，ID:${id}`)

  const playlist = musicStore.playlists.find(p => p.id === id)
  if (!playlist) {
    console.error('未找到要删除的歌单')
    message.error('未找到要删除的歌单')
    return
  }

  info(`找到歌单:${playlist.name}`)

  // 如果删除的是当前播放列表，先停止播放
  if (musicStore.currentPlaylistId === id && musicStore.isPlaying) {
    info('停止当前播放')
    musicStore.pause()
  }

  // 从折叠状态中移除
  const collapseIndex = activeCollapseKeys.value.indexOf(id)
  if (collapseIndex > -1) {
    activeCollapseKeys.value.splice(collapseIndex, 1)
    info('从折叠状态中移除')
  }

  info('调用 store 删除方法')
  musicStore.deletePlaylist(id)
  message.success(`歌单 "${playlist.name}" 已删除`)
}

// 清空歌单（新增功能）
function handleClearPlaylist(playlistId: string) {
  const playlist = musicStore.playlists.find(p => p.id === playlistId)
  if (!playlist) return

  Modal.confirm({
    title: '清空歌单',
    content: `确定要清空歌单 "${playlist.name}" 中的所有歌曲吗？此操作无法撤销。`,
    okText: '确定清空',
    okType: 'danger',
    cancelText: '取消',
    onOk() {
      // 如果正在播放这个歌单中的歌曲，先停止播放
      if (musicStore.currentPlaylistId === playlistId && musicStore.currentSong) {
        musicStore.pause()
        musicStore.currentSong = null
      }

      // 清空歌单
      playlist.songs = []
      message.success(`歌单 "${playlist.name}" 已清空`)
    },
  })
}

function handlePlaySong(song: Song, playlistId: string) {
  console.error('点击播放歌曲:', song.title, song.url)
  musicStore.playSong(song, playlistId)
}

function handleRemoveFromPlaylist(playlistId: string, songId: string) {
  if (musicStore.removeSongFromPlaylist(playlistId, songId)) {
    message.success('已从歌单中移除')
  }
}

function handleTogglePlay() {
  console.error('切换播放状态，当前:', musicStore.isPlaying)
  musicStore.togglePlay()
}

function toggleRepeatMode() {
  const modes: Array<'none' | 'single' | 'all'> = ['none', 'single', 'all']
  const currentIndex = modes.indexOf(musicStore.repeatMode)
  const nextIndex = (currentIndex + 1) % modes.length
  musicStore.repeatMode = modes[nextIndex]
  message.info(`播放模式: ${repeatModeText.value}`)
}

function toggleShuffleMode() {
  musicStore.shuffleMode = !musicStore.shuffleMode
  message.info(musicStore.shuffleMode ? '随机播放已开启' : '随机播放已关闭')
}

function resetAddSongModal() {
  selectedFiles.value = []
  selectedPlaylistId.value = 'default'
}
// 重命名相关状态
const renamingPlaylistId = ref<string | null>(null)
const renamedPlaylistName = ref('')

// 开始重命名 - 添加自动聚焦
async function startRenaming(playlist: Playlist) {
  renamingPlaylistId.value = playlist.id
  renamedPlaylistName.value = playlist.name

  // 等待 DOM 更新后聚焦输入框
  await nextTick()
  // 查找当前重命名的输入框并聚焦
  const inputElement = document.querySelector(`#rename-input-${playlist.id}`) as HTMLInputElement
  if (inputElement) {
    inputElement.focus()
    // 选中所有文本
    inputElement.select()
    info(`输入框已聚焦:${playlist.name}`)
  } else {
    error('未找到输入框元素')
  }
}

// 确认重命名 - 修改为支持失焦触发
function confirmRename() {
  if (!renamingPlaylistId.value) {
    return
  }

  // 如果输入为空或只有空格，恢复原名称并取消重命名
  if (!renamedPlaylistName.value.trim()) {
    cancelRename()
    return
  }

  const playlist = musicStore.playlists.find(p => p.id === renamingPlaylistId.value)
  if (!playlist) {
    cancelRename()
    return
  }

  // 如果名称没有变化，直接取消
  if (renamedPlaylistName.value.trim() === playlist.name) {
    cancelRename()
    return
  }

  // 检查是否与其他歌单重名
  const duplicateName = musicStore.playlists.some(p =>
    p.id !== renamingPlaylistId.value
    && p.name.trim().toLowerCase() === renamedPlaylistName.value.trim().toLowerCase(),
  )

  if (duplicateName) {
    message.error('歌单名称已存在，请使用其他名称')
    // 重名时不取消重命名状态，让用户重新输入
    return
  }

  // 执行重命名
  const oldName = playlist.name
  playlist.name = renamedPlaylistName.value.trim()
  message.success(`歌单已从 "${oldName}" 重命名为 "${playlist.name}"`)

  cancelRename()
}

// 取消重命名
function cancelRename() {
  renamingPlaylistId.value = null
  renamedPlaylistName.value = ''
}

// 处理重命名输入框的回车事件
function handleRenameKeydown(event: KeyboardEvent) {
  if (event.key === 'Enter') {
    confirmRename()
  } else if (event.key === 'Escape') {
    cancelRename()
  }
}

// 处理失焦事件
function handleRenameBlur() {
  // 失焦时自动执行重命名
  confirmRename()
}
</script>

<template>
  <div class="music-player">
    <!-- 当前播放信息 -->
    <ProList title="">
      <!-- 播放进度 -->
      <ProList title="播放进度">
        <ProListItem
          :title="musicStore.currentSong?.title || '未选择歌曲'"
          vertical
        >
          <div class="mb-1 flex items-center justify-between text-xs text-color-3">
            <span>{{ currentDuration }}</span>
            <span>{{ totalDuration }}</span>
          </div>
          <Slider
            class="mb-4"
            :max="musicStore.currentSong?.duration || 100"
            :on-after-change="onProgressSliderAfterChange"
            :step="0.1"
            :value="localCurrentTime"
            @change="onProgressSliderChange"
            @mousedown="onProgressSliderStart"
            @touchstart="onProgressSliderStart"
          />

          <div class="flex items-center justify-center gap-4">
            <Button
              size="small"
              :title="repeatModeText"
              type="text"
              @click="toggleRepeatMode"
            >
              <template #icon>
                <div :class="`${repeatModeIcon} text-lg`" />
              </template>
            </Button>
            <Button
              :disabled="!musicStore.currentPlaylist?.songs.length"
              type="text"
              @click="musicStore.prevSong"
            >
              <template #icon>
                <StepBackwardOutlined class="text-xl" />
              </template>
            </Button>

            <Button
              :disabled="!musicStore.currentSong"
              shape="circle"
              size="large"
              type="primary"
              @click="handleTogglePlay"
            >
              <template #icon>
                <PauseCircleOutlined
                  v-if="musicStore.isPlaying"
                  class="text-2xl"
                />
                <PlayCircleOutlined
                  v-else
                  class="text-2xl"
                />
              </template>
            </Button>

            <Button
              :disabled="!musicStore.currentPlaylist?.songs.length"
              type="text"
              @click="musicStore.nextSong"
            >
              <template #icon>
                <StepForwardOutlined class="text-xl" />
              </template>
            </Button>
            <Button
              :class="{ 'text-color-primary-5': musicStore.shuffleMode }"
              size="small"
              :title="musicStore.shuffleMode ? '随机播放' : '顺序播放'"
              type="text"
              @click="toggleShuffleMode"
            >
              <template #icon>
                <div class="i-solar-shuffle-bold text-lg" />
              </template>
            </Button>
          </div>
        </ProListItem>
      </ProList>

      <!-- 音量控制 -->
      <ProList title="音频设置">
        <ProListItem
          description="调整播放音量"
          title="音量"
          vertical
        >
          <div class="flex items-center gap-3">
            <SoundOutlined class="text-color-3" />
            <Slider
              class="flex-1"
              :max="100"
              :min="0"
              :on-after-change="onVolumeSliderAfterChange"
              :value="localVolume"
              @change="onVolumeSliderChange"
              @mousedown="onVolumeSliderStart"
              @touchstart="onVolumeSliderStart"
            />
            <span class="w-10 text-right text-xs text-color-3">
              {{ Math.round(localVolume) }}%
            </span>
          </div>
        </ProListItem>
      </ProList>

      <!-- 歌单管理 -->
      <ProList title="歌单管理">
        <div class="mb-4">
          <Button
            class="mr-2"
            type="primary"
            @click="showCreatePlaylistModal = true"
          >
            <template #icon>
              <PlusOutlined />
            </template>
            创建歌单
          </Button>

          <Button
            class="mr-2"
            type="default"
            @click="showAddSongModal = true; resetAddSongModal()"
          >
            <template #icon>
              <FolderOpenOutlined />
            </template>
            添加本地音乐
          </Button>
          <Button
            class="mr-2"
            @click="activeCollapseKeys = musicStore.playlists.map(p => p.id)"
          >
            展开全部
          </Button>
          <Button
            class="mr-2"
            @click="activeCollapseKeys = []"
          >
            折叠全部
          </Button>
        </div>

        <Collapse
          v-model:active-key="activeCollapseKeys"
          :bordered="false"
          class="music-playlist-collapse"
          expand-icon-position="end"
        >
          <Collapse.Panel
            v-for="playlist in musicStore.playlists"
            :key="playlist.id"
            class="mb-2"
          >
            <template #header>
              <div class="w-full flex items-center justify-between">
                <div class="flex items-center gap-2">
                  <!-- 当前播放指示器 -->
                  <div
                    v-if="musicStore.currentPlaylistId === playlist.id"
                    class="bg-color-primary-5 h-2 w-2 animate-pulse rounded-full"
                  />
                  <div
                    v-else
                    class="w-2"
                  />

                  <!-- 歌单信息 - 所有歌单都支持重命名 -->
                  <div class="flex-1">
                    <!-- 重命名模式 -->
                    <div
                      v-if="renamingPlaylistId === playlist.id"
                      @click.stop
                    >
                      <Input
                        :id="`rename-input-${playlist.id}`"
                        v-model:value="renamedPlaylistName"
                        class="playlist-rename-input"
                        :maxlength="50"
                        :placeholder="playlist.id === 'default' ? '默认歌单名称' : '歌单名称'"
                        size="small"
                        @blur="handleRenameBlur"
                        @keydown="handleRenameKeydown"
                      />
                    </div>

                    <!-- 正常显示模式 -->
                    <div
                      v-else
                      class="group"
                    >
                      <div
                        class="cursor-pointer hover:text-color-primary-5 font-medium transition-colors"
                      >
                        {{ playlist.name }}
                      </div>
                      <div class="text-xs text-color-3">
                        {{ playlist.songs.length }} 首歌曲
                        <span v-if="playlist.songs.length > 0">
                          · {{ formatTotalDuration(playlist.songs) }}
                        </span>
                      </div>
                    </div>
                  </div>
                </div>
                <!-- 歌单操作按钮 -->
                <div
                  class="w-32 flex items-center justify-end gap-1"
                >
                  <!-- 重命名模式的确认/取消按钮 -->
                  <template v-if="renamingPlaylistId === playlist.id">
                    <Button
                      size="small"
                      title="确认重命名 (Enter)"
                      type="text"
                      @click="confirmRename"
                    >
                      <template #icon>
                        <div class="i-solar-check-circle-bold text-green-500" />
                      </template>
                    </Button>
                    <Button
                      size="small"
                      title="取消重命名 (Esc)"
                      type="text"
                      @click="cancelRename"
                    >
                      <template #icon>
                        <div class="i-solar-close-circle-bold text-red-500" />
                      </template>
                    </Button>
                  </template>
                  <!-- 正常模式的操作按钮 -->
                  <template v-else>
                    <Button
                      :disabled="playlist.songs.length === 0"
                      size="small"
                      title="播放歌单"
                      type="text"
                      @click="handlePlayPlaylist(playlist)"
                    >
                      <template #icon>
                        <PlayCircleOutlined />
                      </template>
                    </Button>
                    <Button
                      size="small"
                      title="重命名"
                      type="text"
                      @click="startRenaming(playlist)"
                    >
                      <template #icon>
                        <EditOutlined />
                      </template>
                    </Button>

                    <!-- 清空歌单按钮 -->
                    <Button
                      v-if="playlist.id === 'default'"
                      :disabled="playlist.songs.length === 0"
                      size="small"
                      title="清空歌单"
                      type="text"
                      @click="handleClearPlaylist(playlist.id)"
                    >
                      <template #icon>
                        <div class="i-solar-trash-bin-minimalistic-2-linear text-base" />
                      </template>
                    </Button>

                    <!-- 删除歌单按钮 -->
                    <Button
                      v-if="playlist.id !== 'default'"
                      danger
                      size="small"
                      title="删除歌单"
                      type="text"
                      @click.stop="confirmDeletePlaylist(playlist)"
                    >
                      <template #icon>
                        <DeleteOutlined />
                      </template>
                    </Button>
                  </template>
                </div>
              </div>
            </template>

            <!-- 歌曲列表 -->
            <div
              v-if="playlist.songs.length > 0"
              class="mt-2"
            >
              <List
                :data-source="playlist.songs"
                size="small"
                :split="false"
              >
                <template #renderItem="{ item, index }">
                  <List.Item class="song-item">
                    <div class="w-full flex items-center justify-between">
                      <div class="min-w-0 flex flex-1 items-center gap-3">
                        <!-- 歌曲序号 -->
                        <div class="w-6 text-center text-xs text-color-3">
                          {{ index + 1 }}
                        </div>

                        <!-- 播放状态指示器 -->
                        <div class="w-4 flex justify-center">
                          <div
                            v-if="musicStore.currentSong?.id === item.id && musicStore.isPlaying"
                            class="i-solar-music-notes-linear animate-pulse text-color-primary-5"
                          />
                          <div
                            v-else-if="musicStore.currentSong?.id === item.id"
                            class="i-solar-music-notes-linear text-color-primary-5"
                          />
                          <div
                            v-else
                            class="i-solar-music-notes-linear text-color-3"
                          />
                        </div>

                        <!-- 歌曲信息 -->
                        <div class="min-w-0 flex-1">
                          <div
                            class="cursor-pointer truncate text-sm hover:text-color-primary-5 font-medium"
                            :class="musicStore.currentSong?.id === item.id ? 'text-color-primary-5' : ''"
                            :title="item.title"
                            @click="handlePlaySong(item, playlist.id)"
                          >
                            {{ item.title }}
                          </div>
                          <div class="truncate text-xs text-color-3">
                            <span
                              v-if="item.duration"
                              class="ml-2"
                            >
                              · {{ formatTime(item.duration) }}
                            </span>
                          </div>
                        </div>
                      </div>

                      <!-- 歌曲操作按钮 -->
                      <div class="flex flex-shrink-0 items-center gap-1">
                        <Button
                          class="hover-show opacity-0 transition-opacity"
                          size="small"
                          title="播放歌曲"
                          type="text"
                          @click="handlePlaySong(item, playlist.id)"
                        >
                          <template #icon>
                            <PlayCircleOutlined />
                          </template>
                        </Button>

                        <Button
                          class="hover-show opacity-0 transition-opacity"
                          danger
                          size="small"
                          title="从歌单中移除"
                          type="text"
                          @click="handleRemoveFromPlaylist(playlist.id, item.id)"
                        >
                          <template #icon>
                            <div class="i-solar-close-circle-bold text-sm" />
                          </template>
                        </Button>
                      </div>
                    </div>
                  </List.Item>
                </template>
              </List>
            </div>

            <!-- 空歌单提示 -->
            <div
              v-else
              class="py-4 text-center text-color-3"
            >
              <div class="i-solar-music-notes-linear mb-2 text-2xl opacity-50" />
              <div class="text-sm">
                歌单为空
              </div>
              <div class="mt-1 text-xs">
                点击上方"添加本地音乐"来添加歌曲
              </div>
            </div>
          </Collapse.Panel>
        </Collapse>
      </ProList>
    </prolist>
  </div>

  <!-- 创建歌单弹窗 -->
  <Modal
    v-model:open="showCreatePlaylistModal"
    :ok-button-props="{ disabled: !newPlaylistName.trim() }"
    title="创建新歌单"
    @ok="handleCreatePlaylist"
  >
    <Input
      v-model:value="newPlaylistName"
      placeholder="请输入歌单名称"
      @press-enter="handleCreatePlaylist"
    />
  </Modal>

  <!-- 添加本地音乐弹窗 -->
  <Modal
    v-model:open="showAddSongModal"
    :ok-button-props="{ disabled: selectedFiles.length === 0 }"
    title="添加本地音乐"
    width="600px"
    @ok="handleAddSelectedSongs"
  >
    <div class="space-y-4">
      <div class="mb-4 text-sm text-color-3">
        支持的音频格式：MP3、WAV、FLAC、AAC、M4A、OGG、WMA
      </div>

      <!-- 选择歌单 -->
      <div>
        <div class="mb-2 text-sm font-medium">
          选择目标歌单：
        </div>
        <Select
          v-model:value="selectedPlaylistId"
          class="w-full"
          :options="playlistOptions"
          placeholder="请选择要添加到的歌单"
        />
      </div>

      <!-- 选择文件按钮 -->
      <Button
        block
        class="mb-4"
        @click="handleSelectLocalFiles"
      >
        <template #icon>
          <FolderOpenOutlined />
        </template>
        选择音乐文件
      </Button>

      <!-- 已选择的文件列表 -->
      <div
        v-if="selectedFiles.length > 0"
        class="space-y-2"
      >
        <div class="text-sm font-medium">
          已选择 {{ selectedFiles.length }} 个文件：
        </div>
        <div class="border-color-2 max-h-40 overflow-y-auto border rounded p-2">
          <div
            v-for="(file, index) in selectedFiles"
            :key="index"
            class="mb-1 truncate rounded bg-color-1 p-2 text-xs text-color-3 last:mb-0"
            :title="file"
          >
            {{ file.split(/[/\\]/).pop() }}
          </div>
        </div>
      </div>

      <!-- 快速创建歌单 -->
      <div class="border-color-2 border-t pt-4">
        <div class="mb-2 text-sm text-color-3">
          或者创建新歌单：
        </div>
        <div class="flex gap-2">
          <Input
            v-model:value="newPlaylistName"
            placeholder="新歌单名称"
            @press-enter="handleCreatePlaylist"
          />
          <Button
            :disabled="!newPlaylistName.trim()"
            @click="handleCreatePlaylist"
          >
            创建
          </Button>
        </div>
      </div>
    </div>
  </Modal>
</template>

<style scoped>
.music-player {
  user-select: none;
}

.music-player .ant-slider-handle {
  border-color: var(--ant-primary-color);
}

.music-player .ant-slider-track {
  background-color: var(--ant-primary-color);
}
/* 自定义折叠面板样式 */
.music-playlist-collapse :deep(.ant-collapse-item) {
  border: 1px solid var(--ant-border-color-base);
  border-radius: 8px;
  overflow: hidden;
  margin-bottom: 8px;
}

.music-playlist-collapse :deep(.ant-collapse-header) {
  padding: 12px 16px;
  background-color: var(--ant-background-color-light);
}

.music-playlist-collapse :deep(.ant-collapse-content-box) {
  padding: 0 16px 12px;
  background-color: var(--ant-component-background);
}

/* 歌曲项样式 */
.song-item {
  padding: 8px 12px;
  border-radius: 6px;
  transition: background-color 0.2s;
  position: relative;
}

.song-item:hover {
  background-color: var(--ant-background-color-light);
}

/* 悬停时显示操作按钮 */
.song-item:hover .hover-show {
  opacity: 1;
}

.hover-show {
  opacity: 0;
  transition: opacity 0.2s;
}

/* 动画效果 */
@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}

.animate-pulse {
  animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

/* 重命名输入框样式 */
.playlist-rename-input {
  width: 200px;
  font-weight: 500;
}

.playlist-rename-input :deep(.ant-input) {
  padding: 2px 6px;
  border-radius: 4px;
  font-weight: 500;
}

/* 组内元素的过渡效果 */
.group .transition-opacity {
  transition: opacity 0.2s ease-in-out;
}

/* Collapse 面板悬停效果 */
.music-playlist-collapse :deep(.ant-collapse-header) {
  position: relative;
}

.music-playlist-collapse :deep(.ant-collapse-item:hover .ant-collapse-header) {
  background-color: var(--ant-background-color-light);
}
</style>
