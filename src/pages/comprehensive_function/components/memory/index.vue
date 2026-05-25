<script setup lang="ts">
import { DeleteOutlined, DownloadOutlined } from '@ant-design/icons-vue'
import { Button, Dropdown, message, Modal, Spin } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import { ref, onMounted } from 'vue'

import { useConfigStore } from '@/stores/config'

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
}

interface WeeklySummary {
  week: string
  date_range: string
  keywords: string[]
  emotion_arc: string[]
  summary: string
  important_events: string[]
  chat_count: number
}

interface MonthlySummary {
  month: string
  emotion_distribution: Record<string, number>
  topics: string[]
  relationship_growth: string
  milestones: string[]
}

const configStore = useConfigStore()
const loading = ref(false)
const userName = ref('我')
const memoryInfo = ref<MemoryInfo | null>(null)
const todayChats = ref<DayChat[]>([])
const weekChats = ref<DayChat[]>([])
const monthChats = ref<DayChat[]>([])
const allChats = ref<DayChat[]>([])
const weeklySummaries = ref<WeeklySummary[]>([])
const monthlySummaries = ref<MonthlySummary[]>([])
const expandedSections = ref<Set<string>>(new Set(['today']))
const selectedChat = ref<{ date: string; messages: ChatMessage[] } | null>(null)

async function loadAll() {
  const charId = configStore.currentCharacterId
  loading.value = true
  try {
    const dates = await invoke<string[]>('get_chat_dates', { characterId: charId })
    const today = new Date().toISOString().split('T')[0]
    const weekAgo = new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString().split('T')[0]
    const monthAgo = new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString().split('T')[0]

    const all: DayChat[] = []
    for (const date of dates) {
      const chat = await invoke<DayChat>('get_chat_by_date', { characterId: charId, date })
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

onMounted(async () => {
  await configStore.init()
  // Load user profile for display name
  try {
    const profile = await invoke<any>('get_user_profile', { characterId: configStore.currentCharacterId })
    userName.value = profile?.user_name || '我'
  } catch {
    userName.value = '我'
  }
  memoryInfo.value = await invoke<MemoryInfo>('get_character_memory_info', { characterId: configStore.currentCharacterId })
  // Load weekly and monthly summaries
  try {
    weeklySummaries.value = await invoke<WeeklySummary[]>('get_weekly_summaries', { characterId: configStore.currentCharacterId })
    monthlySummaries.value = await invoke<MonthlySummary[]>('get_monthly_summaries', { characterId: configStore.currentCharacterId })
  } catch (err) {
    console.error('Failed to load summaries:', err)
  }
  await loadAll()
})

async function loadChatForDate(date: string) {
  // If clicking the already selected chat, toggle close it
  if (selectedChat.value?.date === date) {
    selectedChat.value = null
    return
  }
  loading.value = true
  try {
    selectedChat.value = await invoke<DayChat>('get_chat_by_date', {
      characterId: configStore.currentCharacterId,
      date,
    })
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
    const markdown = await invoke<string>('export_chats_markdown', {
      characterId: configStore.currentCharacterId,
    })
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

function handleClearAction(range: string) {
  const charName = configStore.currentCharacter?.name || '当前'
  const rangeLabel: Record<string, string> = {
    today: '今天',
    week: '本周',
    month: '本月',
    all: '全部',
  }
  Modal.confirm({
    title: `清空${rangeLabel[range]}记忆`,
    content: `将删除「${charName}」角色的${rangeLabel[range]}记忆，是否继续？`,
    okText: '确认删除',
    okType: 'danger',
    async onOk() {
      try {
        if (range === 'all') {
          await invoke('clear_all_chats', { characterId: configStore.currentCharacterId })
        } else {
          await invoke('clear_chat_by_range', {
            characterId: configStore.currentCharacterId,
            range,
          })
        }
        message.success('记忆已清空')
        selectedChat.value = null
        memoryInfo.value = await invoke<MemoryInfo>('get_character_memory_info', {
          characterId: configStore.currentCharacterId,
        })
        await loadAll()
      } catch (err) {
        console.error('Failed to clear:', err)
        message.error('清空失败')
      }
    },
  })
}

const clearMenuItems = [
  { key: 'today', label: '清空今天记忆' },
  { key: 'week', label: '清空本周记忆' },
  { key: 'month', label: '清空本月记忆' },
  { key: 'all', label: '清空全部记忆' },
]

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
      <!-- 角色标识 -->
      <div class="character-badge">
        <span class="character-name">{{ configStore.currentCharacter?.name || '未选中角色' }}</span>
        <span class="character-hint">的记忆</span>
      </div>

      <!-- 统计卡片 -->
      <div v-if="memoryInfo" class="stats-row">
        <div class="stat-card">
          <div class="stat-value">{{ memoryInfo.chat_days }}</div>
          <div class="stat-label">对话天数</div>
        </div>
        <div class="stat-card clickable" @click="toggleSection('weekly')">
          <div class="stat-value">{{ memoryInfo.weekly_summaries }}</div>
          <div class="stat-label">周总结 ↙</div>
        </div>
        <div class="stat-card clickable" @click="toggleSection('monthly')">
          <div class="stat-value">{{ memoryInfo.monthly_summaries }}</div>
          <div class="stat-label">月总结 ↙</div>
        </div>
      </div>

      <!-- 周总结详情 -->
      <div class="section-card" style="margin-bottom: 12px;">
        <div class="section-header" @click="toggleSection('weekly')">
          <span class="section-title">📝 周总结详情</span>
          <span class="section-count">{{ weeklySummaries.length }} 篇</span>
          <span class="expand-icon">{{ expandedSections.has('weekly') ? '▼' : '▶' }}</span>
        </div>
        <div v-if="expandedSections.has('weekly')" class="section-content">
          <div v-if="weeklySummaries.length === 0" class="empty-text">暂无周总结</div>
          <div v-for="ws in weeklySummaries" :key="ws.week" class="summary-card">
            <div class="summary-header">
              <span class="summary-title">{{ ws.week }}</span>
              <span class="summary-sub">{{ ws.date_range }}</span>
            </div>
            <div class="summary-content">{{ ws.summary }}</div>
            <div v-if="ws.keywords.length > 0" class="summary-tags">
              <span v-for="kw in ws.keywords" :key="kw" class="tag">{{ kw }}</span>
            </div>
            <div v-if="ws.important_events.length > 0" class="summary-events">
              <div class="events-label">重要事件：</div>
              <div v-for="ev in ws.important_events" :key="ev" class="event-item">• {{ ev }}</div>
            </div>
          </div>
        </div>
      </div>

      <!-- 月总结详情 -->
      <div class="section-card" style="margin-bottom: 12px;">
        <div class="section-header" @click="toggleSection('monthly')">
          <span class="section-title">📊 月总结详情</span>
          <span class="section-count">{{ monthlySummaries.length }} 篇</span>
          <span class="expand-icon">{{ expandedSections.has('monthly') ? '▼' : '▶' }}</span>
        </div>
        <div v-if="expandedSections.has('monthly')" class="section-content">
          <div v-if="monthlySummaries.length === 0" class="empty-text">暂无月总结</div>
          <div v-for="ms in monthlySummaries" :key="ms.month" class="summary-card">
            <div class="summary-header">
              <span class="summary-title">{{ ms.month }}</span>
            </div>
            <div v-if="ms.topics.length > 0" class="summary-tags">
              <span v-for="topic in ms.topics" :key="topic" class="tag">{{ topic }}</span>
            </div>
            <div v-if="ms.relationship_growth" class="summary-content">{{ ms.relationship_growth }}</div>
            <div v-if="ms.milestones.length > 0" class="summary-events">
              <div class="events-label">里程碑：</div>
              <div v-for="m in ms.milestones" :key="m" class="event-item">• {{ m }}</div>
            </div>
          </div>
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

      <!-- 聊天详情（可折叠） -->
      <div v-if="selectedChat" class="chat-detail-card">
        <div class="chat-detail-header" @click="selectedChat = null">
          <div>
            <span class="chat-detail-date">{{ selectedChat.date }}</span>
            <span class="chat-detail-count">{{ selectedChat.messages.length }} 条消息</span>
          </div>
          <span class="collapse-hint">点击收起</span>
        </div>
        <div class="chat-messages">
          <div
            v-for="msg in selectedChat.messages"
            :key="msg.id"
            class="chat-message"
            :class="msg.role"
          >
            <div class="msg-header">
              <span class="msg-role">{{ msg.role === 'user' ? userName : configStore.currentCharacter?.name || 'Bongo' }}</span>
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
        <Dropdown>
          <Button danger>
            <template #icon><DeleteOutlined /></template>
            清空记忆
          </Button>
          <template #overlay>
            <div class="clear-dropdown">
              <div
                v-for="item in clearMenuItems"
                :key="item.key"
                class="clear-dropdown-item"
                @click="handleClearAction(item.key)"
              >
                {{ item.label }}
              </div>
            </div>
          </template>
        </Dropdown>
      </div>
    </Spin>
  </div>
</template>

<style scoped>
.memory-container {
  padding: 16px;
  user-select: none;
}

.character-badge {
  display: flex;
  align-items: baseline;
  gap: 4px;
  margin-bottom: 16px;
}

.character-name {
  font-size: 16px;
  font-weight: bold;
  color: #333;
}

.character-hint {
  font-size: 13px;
  color: #999;
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

.stat-card.clickable {
  cursor: pointer;
  transition: background-color 0.15s;
}

.stat-card.clickable:hover {
  background: #e8e8e8;
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
  cursor: pointer;
}

.chat-detail-date {
  font-weight: bold;
  font-size: 14px;
  color: #333;
}

.chat-detail-count {
  font-size: 12px;
  color: #999;
  margin-left: 8px;
}

.collapse-hint {
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

.clear-dropdown {
  background: white;
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  padding: 4px 0;
  min-width: 140px;
}

.clear-dropdown-item {
  padding: 8px 16px;
  cursor: pointer;
  font-size: 13px;
  color: #333;
  transition: background-color 0.15s;
}

.clear-dropdown-item:hover {
  background: #f5f5f5;
}

.clear-dropdown-item:last-child {
  color: #ff4d4f;
}

.summary-card {
  padding: 12px 16px;
  border-bottom: 1px solid #f0f0f0;
}

.summary-card:last-child {
  border-bottom: none;
}

.summary-header {
  display: flex;
  align-items: baseline;
  gap: 8px;
  margin-bottom: 8px;
}

.summary-title {
  font-weight: bold;
  font-size: 14px;
  color: #333;
}

.summary-sub {
  font-size: 12px;
  color: #999;
}

.summary-content {
  font-size: 13px;
  color: #555;
  line-height: 1.6;
  white-space: pre-wrap;
  margin-bottom: 8px;
}

.summary-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-bottom: 8px;
}

.tag {
  background: #e6f7ff;
  color: #1890ff;
  border-radius: 4px;
  padding: 2px 8px;
  font-size: 12px;
}

.summary-events {
  margin-top: 6px;
}

.events-label {
  font-size: 12px;
  color: #999;
  margin-bottom: 4px;
}

.event-item {
  font-size: 13px;
  color: #555;
  padding-left: 4px;
}
</style>