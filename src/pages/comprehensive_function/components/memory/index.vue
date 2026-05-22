<script setup lang="ts">
import { DeleteOutlined, DownloadOutlined } from '@ant-design/icons-vue'
import { Button, message, Modal, Spin } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import { ref, onMounted, computed } from 'vue'

interface ChatMessage {
  id: string
  role: string
  content: string
  timestamp: number
}

interface DayChat {
  date: string
  messages: ChatMessage[]
}

interface MemoryInfo {
  chat_days: number
  weekly_summaries: number
  monthly_summaries: number
  storage_path: string
}

const loading = ref(false)
const memoryInfo = ref<MemoryInfo | null>(null)
const todayChats = ref<DayChat[]>([])
const weekChats = ref<DayChat[]>([])
const monthChats = ref<DayChat[]>([])
const allChats = ref<DayChat[]>([])
const expandedSections = ref<Set<string>>(new Set(['today']))
const selectedChat = ref<{ date: string; messages: ChatMessage[] } | null>(null)

onMounted(async () => {
  await loadMemoryInfo()
  await loadAllChats()
})

async function loadMemoryInfo() {
  try {
    memoryInfo.value = await invoke<MemoryInfo>('get_memory_info')
  } catch (err) {
    console.error('Failed to load memory info:', err)
  }
}

async function loadAllChats() {
  loading.value = true
  try {
    const dates = await invoke<string[]>('get_chat_dates')
    const today = new Date().toISOString().split('T')[0]
    const weekAgo = new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString().split('T')[0]
    const monthAgo = new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString().split('T')[0]

    const all: DayChat[] = []
    for (const date of dates) {
      const chat = await invoke<DayChat>('get_chat_by_date', { date })
      all.push(chat)
    }

    todayChats.value = all.filter(c => c.date >= today)
    weekChats.value = all.filter(c => c.date >= weekAgo)
    monthChats.value = all.filter(c => c.date >= monthAgo)
    allChats.value = all
  } catch (err) {
    console.error('Failed to load chats:', err)
  } finally {
    loading.value = false
  }
}

async function loadChatForDate(date: string) {
  loading.value = true
  try {
    selectedChat.value = await invoke<DayChat>('get_chat_by_date', { date })
  } catch (err) {
    console.error('Failed to load chat:', err)
    message.error('加载对话失败')
  } finally {
    loading.value = false
  }
}

function toggleSection(key: string) {
  if (expandedSections.value.has(key)) {
    expandedSections.value.delete(key)
  } else {
    expandedSections.value.add(key)
  }
  expandedSections.value = new Set(expandedSections.value)
}

function formatTime(timestamp: number): string {
  return new Date(timestamp).toLocaleTimeString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
  })
}

async function handleExport() {
  try {
    const markdown = await invoke<string>('export_chats_markdown')
    const blob = new Blob([markdown], { type: 'text/markdown' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `bongo-chat-${new Date().toISOString().split('T')[0]}.md`
    a.click()
    URL.revokeObjectURL(url)
    message.success('导出成功')
  } catch (err) {
    console.error('Failed to export:', err)
    message.error('导出失败')
  }
}

async function handleClear() {
  Modal.confirm({
    title: '清空记忆',
    content: '确定要清空所有对话记忆吗？此操作无法撤销。',
    okText: '确定清空',
    okType: 'danger',
    onOk: async () => {
      try {
        await invoke('clear_all_chats')
        message.success('记忆已清空')
        await loadAllChats()
        await loadMemoryInfo()
        selectedChat.value = null
      } catch (err) {
        console.error('Failed to clear:', err)
        message.error('清空失败')
      }
    },
  })
}

function getDateLabel(chat: DayChat): string {
  const today = new Date().toISOString().split('T')[0]
  const yesterday = new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString().split('T')[0]
  if (chat.date === today) return '今天'
  if (chat.date === yesterday) return '昨天'
  return chat.date
}
</script>

<template>
  <div class="memory-container">
    <Spin :spinning="loading">
      <!-- 统计卡片 -->
      <div v-if="memoryInfo" class="stats-row">
        <div class="stat-card">
          <div class="stat-value">{{ memoryInfo.chat_days }}</div>
          <div class="stat-label">对话天数</div>
        </div>
        <div class="stat-card">
          <div class="stat-value">{{ memoryInfo.weekly_summaries }}</div>
          <div class="stat-label">周总结</div>
        </div>
        <div class="stat-card">
          <div class="stat-value">{{ memoryInfo.monthly_summaries }}</div>
          <div class="stat-label">月总结</div>
        </div>
      </div>

      <!-- 时间分组列表 -->
      <div class="section-group">
        <!-- 今天 -->
        <div class="section-card">
          <div class="section-header" @click="toggleSection('today')">
            <span class="section-title">今天</span>
            <span class="section-count">{{ todayChats.length }} 条</span>
            <span class="expand-icon">{{ expandedSections.has('today') ? '▼' : '▶' }}</span>
          </div>
          <div v-if="expandedSections.has('today')" class="section-content">
            <div v-if="todayChats.length === 0" class="empty-text">暂无对话记录</div>
            <div
              v-for="chat in todayChats"
              :key="chat.date"
              class="date-item"
              :class="{ active: selectedChat?.date === chat.date }"
              @click="loadChatForDate(chat.date)"
            >
              <span class="date-label">{{ getDateLabel(chat) }}</span>
              <span class="date-count">{{ chat.messages.length }} 条消息</span>
            </div>
          </div>
        </div>

        <!-- 本周 -->
        <div class="section-card">
          <div class="section-header" @click="toggleSection('week')">
            <span class="section-title">本周</span>
            <span class="section-count">{{ weekChats.length }} 天</span>
            <span class="expand-icon">{{ expandedSections.has('week') ? '▼' : '▶' }}</span>
          </div>
          <div v-if="expandedSections.has('week')" class="section-content">
            <div v-if="weekChats.length === 0" class="empty-text">暂无对话记录</div>
            <div
              v-for="chat in weekChats"
              :key="chat.date"
              class="date-item"
              :class="{ active: selectedChat?.date === chat.date }"
              @click="loadChatForDate(chat.date)"
            >
              <span class="date-label">{{ getDateLabel(chat) }}</span>
              <span class="date-count">{{ chat.messages.length }} 条消息</span>
            </div>
          </div>
        </div>

        <!-- 本月 -->
        <div class="section-card">
          <div class="section-header" @click="toggleSection('month')">
            <span class="section-title">本月</span>
            <span class="section-count">{{ monthChats.length }} 天</span>
            <span class="expand-icon">{{ expandedSections.has('month') ? '▼' : '▶' }}</span>
          </div>
          <div v-if="expandedSections.has('month')" class="section-content">
            <div v-if="monthChats.length === 0" class="empty-text">暂无对话记录</div>
            <div
              v-for="chat in monthChats"
              :key="chat.date"
              class="date-item"
              :class="{ active: selectedChat?.date === chat.date }"
              @click="loadChatForDate(chat.date)"
            >
              <span class="date-label">{{ getDateLabel(chat) }}</span>
              <span class="date-count">{{ chat.messages.length }} 条消息</span>
            </div>
          </div>
        </div>

        <!-- 全部 -->
        <div class="section-card">
          <div class="section-header" @click="toggleSection('all')">
            <span class="section-title">全部</span>
            <span class="section-count">{{ allChats.length }} 天</span>
            <span class="expand-icon">{{ expandedSections.has('all') ? '▼' : '▶' }}</span>
          </div>
          <div v-if="expandedSections.has('all')" class="section-content">
            <div v-if="allChats.length === 0" class="empty-text">暂无对话记录</div>
            <div
              v-for="chat in allChats"
              :key="chat.date"
              class="date-item"
              :class="{ active: selectedChat?.date === chat.date }"
              @click="loadChatForDate(chat.date)"
            >
              <span class="date-label">{{ chat.date }}</span>
              <span class="date-count">{{ chat.messages.length }} 条消息</span>
            </div>
          </div>
        </div>
      </div>

      <!-- 聊天详情 -->
      <div v-if="selectedChat" class="chat-detail-card">
        <div class="chat-detail-header">
          <span class="chat-detail-date">{{ selectedChat.date }}</span>
          <span class="chat-detail-count">{{ selectedChat.messages.length }} 条消息</span>
        </div>
        <div class="chat-messages">
          <div
            v-for="msg in selectedChat.messages"
            :key="msg.id"
            class="chat-message"
            :class="msg.role"
          >
            <div class="msg-header">
              <span class="msg-role">{{ msg.role === 'user' ? '用户' : 'Bongo' }}</span>
              <span class="msg-time">{{ formatTime(msg.timestamp) }}</span>
            </div>
            <div class="msg-content">{{ msg.content }}</div>
          </div>
        </div>
      </div>

      <!-- 操作按钮 -->
      <div class="action-row">
        <Button @click="handleExport">
          <template #icon><DownloadOutlined /></template>
          导出对话
        </Button>
        <Button danger @click="handleClear">
          <template #icon><DeleteOutlined /></template>
          清空记忆
        </Button>
      </div>
    </Spin>
  </div>
</template>

<style scoped>
.memory-container {
  padding: 16px;
  user-select: none;
}

.stats-row {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
}

.stat-card {
  flex: 1;
  background: #f5f5f5;
  border-radius: 8px;
  padding: 12px;
  text-align: center;
  border: 1px solid #e8e8e8;
}

.stat-value {
  font-size: 24px;
  font-weight: bold;
  color: #333;
}

.stat-label {
  font-size: 12px;
  color: #666;
  margin-top: 4px;
}

.section-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 16px;
}

.section-card {
  border: 1px solid #e8e8e8;
  border-radius: 8px;
  overflow: hidden;
}

.section-header {
  display: flex;
  align-items: center;
  padding: 12px 16px;
  background: #fafafa;
  cursor: pointer;
  user-select: none;
  border-bottom: 1px solid transparent;
}

.section-card:has(.section-content[style*="display: block"]) .section-header,
.section-header:has(+ .section-content:not(:empty)) {
  border-bottom-color: #e8e8e8;
}

.section-title {
  font-weight: bold;
  font-size: 14px;
  color: #333;
}

.section-count {
  margin-left: auto;
  margin-right: 8px;
  font-size: 12px;
  color: #999;
}

.expand-icon {
  font-size: 10px;
  color: #999;
}

.section-content {
  background: white;
}

.date-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 16px;
  cursor: pointer;
  border-bottom: 1px solid #f0f0f0;
  transition: background-color 0.15s;
}

.date-item:last-child {
  border-bottom: none;
}

.date-item:hover {
  background: #f5f5f5;
}

.date-item.active {
  background: #e6f7ff;
}

.date-label {
  font-size: 14px;
  color: #333;
}

.date-count {
  font-size: 12px;
  color: #999;
}

.empty-text {
  padding: 16px;
  text-align: center;
  color: #999;
  font-size: 13px;
}

.chat-detail-card {
  border: 1px solid #e8e8e8;
  border-radius: 8px;
  margin-bottom: 16px;
  overflow: hidden;
}

.chat-detail-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  background: #fafafa;
  border-bottom: 1px solid #e8e8e8;
}

.chat-detail-date {
  font-weight: bold;
  font-size: 14px;
  color: #333;
}

.chat-detail-count {
  font-size: 12px;
  color: #999;
}

.chat-messages {
  max-height: 400px;
  overflow-y: auto;
  padding: 12px;
}

.chat-message {
  padding: 10px 12px;
  border-radius: 8px;
  margin-bottom: 8px;
}

.chat-message:last-child {
  margin-bottom: 0;
}

.chat-message.user {
  background: #1890ff;
  color: white;
  margin-left: 20px;
}

.chat-message.assistant {
  background: #f5f5f5;
  color: #333;
  margin-right: 20px;
}

.msg-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 4px;
}

.msg-role {
  font-size: 12px;
  font-weight: bold;
  opacity: 0.8;
}

.msg-time {
  font-size: 11px;
  opacity: 0.6;
}

.msg-content {
  font-size: 13px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
}

.action-row {
  display: flex;
  gap: 8px;
}
</style>