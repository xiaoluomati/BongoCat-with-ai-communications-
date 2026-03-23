<script setup lang="ts">
import { DeleteOutlined, DownloadOutlined, FolderOpenOutlined } from '@ant-design/icons-vue'
import { Button, message, Modal, Tabs } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import { ref, onMounted, computed } from 'vue'

import ProList from '@/components/pro-list/index.vue'
import ProListItem from '@/components/pro-list-item/index.vue'

interface DayChat {
  date: string
  messages: Array<{
    id: string
    role: string
    content: string
    timestamp: number
  }>
}

interface MemoryInfo {
  chat_days: number
  weekly_summaries: number
  monthly_summaries: number
  storage_path: string
}

const activeTab = ref('today')
const chatDates = ref<string[]>([])
const selectedDateChat = ref<DayChat | null>(null)
const memoryInfo = ref<MemoryInfo | null>(null)
const loading = ref(false)

const tabs = [
  { key: 'today', label: '今天' },
  { key: 'week', label: '本周' },
  { key: 'month', label: '本月' },
  { key: 'all', label: '全部' },
]

onMounted(async () => {
  await loadMemoryInfo()
  await loadChatDates()
})

async function loadMemoryInfo() {
  try {
    memoryInfo.value = await invoke<MemoryInfo>('get_memory_info')
  } catch (err) {
    console.error('Failed to load memory info:', err)
  }
}

async function loadChatDates() {
  try {
    chatDates.value = await invoke<string[]>('get_chat_dates')
  } catch (err) {
    console.error('Failed to load chat dates:', err)
  }
}

async function loadDateChat(date: string) {
  loading.value = true
  try {
    selectedDateChat.value = await invoke<DayChat>('get_chat_by_date', { date })
  } catch (err) {
    console.error('Failed to load chat:', err)
    message.error('加载对话失败')
  } finally {
    loading.value = false
  }
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
    
    // Create download
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
        await loadChatDates()
        await loadMemoryInfo()
        selectedDateChat.value = null
      } catch (err) {
        console.error('Failed to clear:', err)
        message.error('清空失败')
      }
    },
  })
}

// Load today's chat by default
loadDateChat(new Date().toISOString().split('T')[0])
</script>

<template>
  <div class="memory-container">
    <ProList>
      <!-- Memory Info -->
      <ProListItem
        v-if="memoryInfo"
        description="存储位置"
        title="记忆统计"
      >
        <div class="text-sm">
          <div>对话天数: {{ memoryInfo.chat_days }} 天</div>
          <div>周总结: {{ memoryInfo.weekly_summaries }} 篇</div>
          <div>月总结: {{ memoryInfo.monthly_summaries }} 篇</div>
        </div>
      </ProListItem>

      <!-- Tabs -->
      <ProListItem title="">
        <Tabs v-model:activeKey="activeTab" type="card">
          <Tabs.TabPane
            v-for="tab in tabs"
            :key="tab.key"
            :tab="tab.label"
          />
        </Tabs>
      </ProListItem>

      <!-- Chat List -->
      <div class="chat-list">
        <div
          v-for="date in chatDates.slice(0, 10)"
          :key="date"
          class="chat-date-item"
          :class="{ active: selectedDateChat?.date === date }"
          @click="loadDateChat(date)"
        >
          {{ date }}
        </div>
        <div v-if="chatDates.length === 0" class="empty">
          暂无对话记录
        </div>
      </div>

      <!-- Selected Chat -->
      <div v-if="selectedDateChat" class="selected-chat">
        <div class="chat-date-header">
          {{ selectedDateChat.date }}
        </div>
        <div
          v-for="msg in selectedDateChat.messages"
          :key="msg.id"
          class="chat-message"
          :class="msg.role"
        >
          <div class="msg-role">
            {{ msg.role === 'user' ? '👤 用户' : '🤖 Bongo' }}
          </div>
          <div class="msg-content">
            {{ msg.content }}
          </div>
          <div class="msg-time">
            {{ formatTime(msg.timestamp) }}
          </div>
        </div>
      </div>

      <!-- Actions -->
      <ProListItem title="">
        <div class="flex gap-2">
          <Button @click="handleExport">
            <template #icon>
              <DownloadOutlined />
            </template>
            导出对话
          </Button>
          <Button danger @click="handleClear">
            <template #icon>
              <DeleteOutlined />
            </template>
            清空记忆
          </Button>
        </div>
      </ProListItem>
    </ProList>
  </div>
</template>

<style scoped>
.memory-container {
  user-select: none;
}

.chat-list {
  max-height: 200px;
  overflow-y: auto;
  padding: 8px;
  border: 1px solid var(--ant-border-color-base);
  border-radius: 8px;
  margin: 8px 0;
}

.chat-date-item {
  padding: 8px 12px;
  cursor: pointer;
  border-radius: 4px;
  transition: background-color 0.2s;
}

.chat-date-item:hover {
  background-color: var(--ant-background-color-light);
}

.chat-date-item.active {
  background-color: var(--ant-primary-color);
  color: white;
}

.empty {
  text-align: center;
  color: #999;
  padding: 20px;
}

.selected-chat {
  max-height: 300px;
  overflow-y: auto;
  padding: 8px;
  border: 1px solid var(--ant-border-color-base);
  border-radius: 8px;
  margin: 8px 0;
}

.chat-date-header {
  font-weight: bold;
  padding: 8px 0;
  border-bottom: 1px solid var(--ant-border-color-base);
  margin-bottom: 8px;
}

.chat-message {
  padding: 8px;
  border-radius: 8px;
  margin-bottom: 8px;
}

.chat-message.user {
  background-color: var(--ant-primary-color);
  color: white;
  margin-left: 20%;
}

.chat-message.assistant {
  background-color: #f0f0f0;
  margin-right: 20%;
}

.msg-role {
  font-size: 12px;
  margin-bottom: 4px;
}

.msg-content {
  line-height: 1.5;
}

.msg-time {
  font-size: 10px;
  opacity: 0.7;
  text-align: right;
  margin-top: 4px;
}
</style>
