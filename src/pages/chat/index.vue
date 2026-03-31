<script setup lang="ts">
import { CloseOutlined, SendOutlined, SettingOutlined } from '@ant-design/icons-vue'
import { Button, Input, Spin } from 'ant-design-vue'
import { getCurrentWebviewWindow, WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref, nextTick, watch } from 'vue'

import { useChatStore } from '@/stores/chat'

const chatStore = useChatStore()
const inputText = ref('')
const messagesContainer = ref<HTMLElement | null>(null)
const chatWindow = getCurrentWebviewWindow()
const currentCharacter = ref('Bongo Cat')
const characterAvatar = ref('')

// 历史消息加载相关
const loadedDates = ref<string[]>([]) // 已加载的日期
const isLoadingHistory = ref(false)
const hasMoreHistory = ref(true)

// Load config and history on mount
onMounted(async () => {
  await chatStore.loadConfig()
  // 先加载今天的消息
  await chatStore.loadHistory()
  loadedDates.value = [getTodayString()]
  
  // Get current character name
  try {
    const config = await invoke<any>('load_config')
    if (config.characters?.current) {
      const char = await invoke<any>('load_character', { id: config.characters.current })
      currentCharacter.value = char.name || 'Bongo Cat'
      characterAvatar.value = char.avatar || ''
    }
  } catch (e) {
    console.error('Failed to load character:', e)
  }
  
  // 滚动到底部
  scrollToBottom()
  
  // 绑定滚动事件监听
  if (messagesContainer.value) {
    messagesContainer.value.addEventListener('scroll', handleScroll)
  }
})

function getTodayString(): string {
  return new Date().toISOString().split('T')[0]
}

// 滚动处理 - 检测是否滚动到顶部
async function handleScroll() {
  if (!messagesContainer.value || isLoadingHistory.value || !hasMoreHistory.value) return
  
  // 距离顶部小于 50px 时加载更多
  if (messagesContainer.value.scrollTop < 50) {
    await loadMoreHistory()
  }
}

// 加载更多历史消息
async function loadMoreHistory() {
  isLoadingHistory.value = true
  
  try {
    // 获取所有有聊天记录的日期
    const allDates = await invoke<string[]>('get_chat_dates')
    
    // 找出还没加载的日期
    const unloadedDates = allDates.filter(date => !loadedDates.value.includes(date))
    
    if (unloadedDates.length === 0) {
      hasMoreHistory.value = false
      isLoadingHistory.value = false
      return
    }
    
    // 按时间倒序取最老的一个（因为 allDates 已经是倒序的，所以取最后一个）
    const oldestDate = unloadedDates[unloadedDates.length - 1]
    
    // 记录当前滚动位置
    const container = messagesContainer.value
    const oldHeight = container?.scrollHeight || 0
    
    // 加载该日期的消息
    const dayChat = await invoke<any>('get_chat_by_date', { date: oldestDate })
    
    if (dayChat?.messages?.length > 0) {
      // 构建日期分隔消息
      const dateDivider = {
        id: `divider_${oldestDate}`,
        role: 'system' as const,
        content: formatDateLabel(oldestDate),
        timestamp: new Date(oldestDate).getTime(),
        isDivider: true
      }
      
      // 将历史消息插入到当前消息列表前面
      const historyMessages = [
        dateDivider,
        ...dayChat.messages.map((msg: any) => ({
          id: msg.id,
          role: msg.role as 'user' | 'assistant',
          content: msg.content,
          timestamp: msg.timestamp
        }))
      ]
      
      chatStore.messages.unshift(...historyMessages)
      loadedDates.value.push(oldestDate)
    }
    
    // 恢复滚动位置
    nextTick(() => {
      if (container) {
        const newHeight = container.scrollHeight
        container.scrollTop = newHeight - oldHeight
      }
    })
    
  } catch (e) {
    console.error('Failed to load more history:', e)
  } finally {
    isLoadingHistory.value = false
  }
}

// 格式化日期显示
function formatDateLabel(dateStr: string): string {
  const today = getTodayString()
  const yesterday = new Date(Date.now() - 86400000).toISOString().split('T')[0]
  
  if (dateStr === today) return '今天'
  if (dateStr === yesterday) return '昨天'
  
  const date = new Date(dateStr)
  const weekDays = ['周日', '周一', '周二', '周三', '周四', '周五', '周六']
  const weekDay = weekDays[date.getDay()]
  
  return `${date.getFullYear()}年${date.getMonth() + 1}月${date.getDate()}日 ${weekDay}`
}

// Auto scroll when messages change (只在发送新消息时滚动到底部)
function scrollToBottom() {
  if (messagesContainer.value) {
    messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
  }
}

// Send message
async function handleSend() {
  if (!inputText.value.trim() || chatStore.isLoading) return
  
  const text = inputText.value.trim()
  inputText.value = ''
  
  await chatStore.sendMessage(text)
  
  // 发送后滚动到底部
  nextTick(() => scrollToBottom())
}

// Handle Enter key
function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    handleSend()
  }
}

// Close window
async function handleClose() {
  await chatWindow.hide()
}

// Open settings
async function handleSettings() {
  // 使用 activate_window 确保设置窗口在最前，同时降低其他窗口层级
  await invoke('activate_window', { windowLabel: 'comprehensive_function' })
  await chatWindow.hide()
}

// Format time
function formatTime(timestamp: number): string {
  const date = new Date(timestamp)
  return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
}
</script>

<template>
  <div class="chat-container">
    <!-- Header -->
    <div class="chat-header" data-tauri-drag-region>
      <Button class="header-btn close" type="text" @click="handleClose">
        <template #icon><CloseOutlined /></template>
      </Button>
      
      <span class="chat-title">{{ currentCharacter }}</span>
      
      <Button class="header-btn settings" type="text" @click="handleSettings">
        <template #icon><SettingOutlined /></template>
      </Button>
    </div>

    <!-- Messages -->
    <div ref="messagesContainer" class="messages-container">
      <!-- 加载更多提示 -->
      <div v-if="isLoadingHistory" class="loading-history">
        <Spin size="small" />
        <span>加载历史消息...</span>
      </div>
      
      <div v-else-if="hasMoreHistory" class="load-more-hint" @click="loadMoreHistory">
        <span>↑ 点击加载更多</span>
      </div>
      
      <div v-else-if="loadedDates.length > 1" class="no-more-hint">
        <span>— 已加载全部历史消息 —</span>
      </div>

      <template v-for="msg in chatStore.messages" :key="msg.id">
        <!-- 日期分隔线 -->
        <div v-if="msg.role === 'system' && msg.isDivider" class="date-divider">
          <span class="date-label">{{ msg.content }}</span>
        </div>
        
        <!-- 普通消息 -->
        <div v-else class="message-wrapper" :class="msg.role">
          <div v-if="msg.role === 'assistant'" class="character-avatar">
            <img v-if="characterAvatar" :src="characterAvatar" alt="avatar">
            <span v-else class="avatar-emoji">🐱</span>
          </div>
          
          <div class="message-bubble">
            <div class="message-content">{{ msg.content }}</div>
            <div class="message-time">{{ formatTime(msg.timestamp) }}</div>
          </div>
        </div>
      </template>

      <!-- Loading -->
      <div v-if="chatStore.isLoading" class="message-wrapper assistant">
        <div class="character-avatar">
          <img v-if="characterAvatar" :src="characterAvatar" alt="avatar">
          <span v-else class="avatar-emoji">🐱</span>
        </div>
        <div class="message-bubble loading">
          <Spin size="small" />
          <span>思考中...</span>
        </div>
      </div>
      
      <!-- 空状态 -->
      <div v-if="chatStore.messages.length === 0 && !isLoadingHistory" class="empty-hint">
        <div>你好，我是 {{ currentCharacter }}</div>
        <div class="text-sm text-gray-400 mt-2">开始我们的对话吧</div>
      </div>
    </div>

    <!-- Input -->
    <div class="input-container">
      <Input
        v-model:value="inputText"
        class="chat-input"
        placeholder="输入消息..."
        :disabled="chatStore.isLoading || !chatStore.enabled"
        @keydown="handleKeydown"
      />
      <Button
        type="primary"
        class="send-btn"
        :disabled="!inputText.trim() || chatStore.isLoading || !chatStore.enabled"
        @click="handleSend"
      >
        <template #icon><SendOutlined /></template>
      </Button>
    </div>
  </div>
</template>

<style scoped>
.chat-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: linear-gradient(180deg, #fff9f5 0%, #fef5ee 100%);
  border-radius: 20px;
  overflow: hidden;
  box-shadow: 0 12px 40px rgba(255, 183, 149, 0.15);
}

/* 头部 */
.chat-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 20px;
  background: rgba(255, 250, 247, 0.85);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  color: #5a4a42;
  -webkit-app-region: drag;
  height: 56px;
  box-sizing: border-box;
  border-bottom: 1px solid rgba(255, 183, 149, 0.15);
  flex-shrink: 0;
}

.chat-title {
  font-weight: 600;
  font-size: 16px;
  font-family: -apple-system, BlinkMacSystemFont, 'PingFang SC', 'Inter', sans-serif;
  color: #6b5a50;
  letter-spacing: -0.2px;
}

.header-btn {
  -webkit-app-region: no-drag;
  color: #b8a090 !important;
  width: 34px;
  height: 34px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  transition: all 0.2s ease;
}

.header-btn:hover {
  background: rgba(255, 154, 122, 0.1) !important;
  color: #ff9a7a !important;
}

/* 消息区域 */
.messages-container {
  flex: 1;
  overflow-y: auto;
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

/* 加载历史提示 */
.loading-history {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 12px;
  color: #c9b5a8;
  font-size: 13px;
}

.load-more-hint {
  text-align: center;
  padding: 12px;
  color: #ff9a7a;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s ease;
  border-radius: 8px;
}

.load-more-hint:hover {
  background: rgba(255, 154, 122, 0.08);
}

.no-more-hint {
  text-align: center;
  padding: 12px;
  color: #d4b5a0;
  font-size: 12px;
}

/* 日期分隔线 */
.date-divider {
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 16px 0;
  position: relative;
}

.date-divider::before {
  content: '';
  position: absolute;
  left: 0;
  right: 0;
  height: 1px;
  background: linear-gradient(90deg, transparent, rgba(255, 183, 149, 0.3), transparent);
}

.date-label {
  background: rgba(255, 250, 247, 0.95);
  padding: 4px 16px;
  border-radius: 12px;
  font-size: 12px;
  color: #b8a090;
  position: relative;
  border: 1px solid rgba(255, 183, 149, 0.15);
}

/* 空状态 */
.empty-hint {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #d4b5a0;
  gap: 12px;
  font-family: -apple-system, BlinkMacSystemFont, 'PingFang SC', 'Inter', sans-serif;
}

.empty-hint .text-sm {
  font-size: 14px;
  color: #e8d5c4;
}

/* 消息 */
.message-wrapper {
  display: flex;
  width: 100%;
}

.message-wrapper.user {
  justify-content: flex-end;
}

.message-wrapper.assistant {
  justify-content: flex-start;
  align-items: flex-start;
  gap: 8px;
}

.character-avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  object-fit: cover;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #f0f0f0;
  overflow: hidden;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

.character-avatar img {
  width: 100%;
  height: 100%;
}

.avatar-emoji {
  font-size: 20px;
  line-height: 1;
}

.message-bubble {
  max-width: 75%;
  padding: 12px 16px;
  border-radius: 18px;
  word-break: break-word;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.04);
  line-height: 1.5;
}

.message-wrapper.user .message-bubble {
  background: linear-gradient(135deg, #ffb899 0%, #ff9a7a 100%);
  color: white;
  border-bottom-right-radius: 4px;
}

.message-wrapper.assistant .message-bubble {
  background: rgba(255, 255, 255, 0.95);
  color: #5a4a42;
  border-bottom-left-radius: 4px;
  border: 1px solid rgba(255, 183, 149, 0.15);
}

.message-content {
  line-height: 1.5;
}

.message-time {
  font-size: 11px;
  opacity: 0.6;
  margin-top: 4px;
  text-align: right;
}

.message-bubble.loading {
  display: flex;
  align-items: center;
  gap: 8px;
  background: rgba(255, 255, 255, 0.95) !important;
  color: #888 !important;
}

/* 输入区域 */
.input-container {
  display: flex;
  gap: 10px;
  padding: 12px 16px 16px;
  background: rgba(255, 250, 247, 0.85);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border-top: 1px solid rgba(255, 183, 149, 0.15);
  flex-shrink: 0;
}

.chat-input {
  flex: 1;
  border-radius: 22px;
  background: rgba(255, 255, 255, 0.95);
  border: 1px solid rgba(255, 183, 149, 0.25);
  padding: 10px 18px;
  font-size: 15px;
  transition: all 0.2s ease;
  color: #5a4a42;
}

.chat-input::placeholder {
  color: #c9b5a8;
}

.chat-input:hover,
.chat-input:focus {
  border-color: rgba(255, 154, 122, 0.5);
  background: white;
  box-shadow: 0 2px 12px rgba(255, 154, 122, 0.1);
}

.send-btn {
  flex-shrink: 0;
  border-radius: 50%;
  width: 42px;
  height: 42px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #ffb899 0%, #ff9a7a 100%);
  border: none;
  box-shadow: 0 3px 12px rgba(255, 154, 122, 0.35);
  transition: all 0.2s ease;
}

.send-btn:hover {
  transform: translateY(-1px);
  box-shadow: 0 5px 16px rgba(255, 154, 122, 0.45);
}
</style>
