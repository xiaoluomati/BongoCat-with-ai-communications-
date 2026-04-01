import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import { useTTSStore } from './tts'

export interface TTSAudioFile {
  seq: number
  path: string
  text: string
}

export interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  timestamp: number
  tts_meta?: {
    date: string
    audio_files: TTSAudioFile[]
  }
}

export const useChatStore = defineStore('chat', () => {
  const messages = ref<ChatMessage[]>([])
  const isLoading = ref(false)
  const enabled = ref(false)
  const maxMessages = ref(100)

  // TTS store instance for streaming
  const ttsStore = useTTSStore()

  // Load config
  async function loadConfig() {
    try {
      const config = await invoke<any>('load_config')
      enabled.value = config.chat?.enabled ?? false
      maxMessages.value = config.chat?.max_messages ?? 100
    } catch (err) {
      console.error('Failed to load chat config:', err)
    }
  }

  // Save message to memory
  async function saveMessageToMemory(message: ChatMessage) {
    try {
      await invoke('save_chat_message', {
        message: {
          id: message.id,
          role: message.role,
          content: message.content,
          timestamp: message.timestamp,
        },
      })
    } catch (err) {
      console.error('Failed to save message to memory:', err)
    }
  }

  // Send message
  async function sendMessage(content: string): Promise<string | null> {
    if (!enabled.value || isLoading.value) return null

    isLoading.value = true

    // Add user message
    const userMessage: ChatMessage = {
      id: `msg_${Date.now()}`,
      role: 'user',
      content,
      timestamp: Date.now(),
    }
    messages.value.push(userMessage)

    // Save to memory
    await saveMessageToMemory(userMessage)

    // Trim messages if needed
    if (messages.value.length > maxMessages.value) {
      messages.value = messages.value.slice(-maxMessages.value)
    }

    try {
      // Check if streaming is enabled
      const config = await invoke<any>('load_config')
      const isStreaming = config.llm?.stream ?? false

      if (isStreaming) {
        // Streaming mode: listen for chunks
        const streamingMessageId = `msg_${Date.now() + 1}`
        let streamingContent = ''
        let _unlistenChunk: UnlistenFn | null = null
        let _unlistenEnd: UnlistenFn | null = null

        // Create placeholder message
        const assistantMessage: ChatMessage = {
          id: streamingMessageId,
          role: 'assistant',
          content: '',
          timestamp: Date.now(),
        }
        messages.value.push(assistantMessage)

        // Listen for chunks and trigger TTS streaming
        await listen<[string, string]>('chat_stream_chunk', (event) => {
          const [, chunk] = event.payload
          streamingContent += chunk
          // Update the message in place
          const msgIndex = messages.value.findIndex(m => m.id === streamingMessageId)
          if (msgIndex !== -1) {
            messages.value[msgIndex].content = streamingContent
          }
          // Trigger TTS streaming
          ttsStore.speakStream(chunk)
        })

        // Send message using stream command (doesn't trigger TTS internally)
        try {
          const response = await invoke<any>('send_message_stream', {
            request: {
              message: content,
              system_prompt: null,
            },
          })

          // Final update
          const msgIndex = messages.value.findIndex(m => m.id === streamingMessageId)
          if (msgIndex !== -1) {
            messages.value[msgIndex].content = response.content
            // Save TTS meta if we have audio files
            if (streamingAudioFiles.length > 0) {
              messages.value[msgIndex].tts_meta = {
                date: today,
                audio_files: streamingAudioFiles
              }
              // Save meta to backend
              try {
                await invoke('save_tts_meta', {
                  msgId: streamingMessageId,
                  date: today,
                  audioFiles: streamingAudioFiles
                })
              } catch (err) {
                console.error('[TTS] save meta error:', err)
              }
            }
          }

          // Save to memory
          await saveMessageToMemory(messages.value[msgIndex])
        } finally {
          // Cleanup listeners would be handled by Tauri automatically
        }

        return streamingContent
      } else {
        // Non-streaming mode: get full response at once
        const response = await invoke<any>('send_message', {
          request: {
            message: content,
            system_prompt: null,
          },
        })

        // Add assistant message
        const assistantMessage: ChatMessage = {
          id: `msg_${Date.now()}`,
          role: 'assistant',
          content: response.content,
          timestamp: Date.now(),
        }
        messages.value.push(assistantMessage)

        // Save to memory
        await saveMessageToMemory(assistantMessage)

        // Trim again
        if (messages.value.length > maxMessages.value) {
          messages.value = messages.value.slice(-maxMessages.value)
        }

        return response.content
      }
    } catch (err) {
      console.error('Failed to send message:', err)
      // Remove user message on error
      messages.value.pop()
      return null
    } finally {
      isLoading.value = false
    }
  }

  // Clear messages (in memory only, not storage)
  function clearMessages() {
    messages.value = []
  }

  // Load today's chat from file
  async function loadHistory() {
    try {
      const todayChat = await invoke<any>('get_today_chat')
      if (todayChat && todayChat.messages) {
        messages.value = todayChat.messages.map((msg: any) => ({
          id: msg.id || `msg_${msg.timestamp}`,
          role: msg.role,
          content: msg.content,
          timestamp: msg.timestamp,
        }))
      } else {
        messages.value = []
      }
    } catch (err) {
      console.error('Failed to load chat history:', err)
      messages.value = []
    }
  }

  // Clear history (from backend)
  async function clearHistory() {
    try {
      await invoke('clear_chat_history')
      messages.value = []
    } catch (err) {
      console.error('Failed to clear chat history:', err)
    }
  }

  // Export all chats
  async function exportAllChats(): Promise<string> {
    try {
      return await invoke<string>('export_all_chats')
    } catch (err) {
      console.error('Failed to export chats:', err)
      return '{}'
    }
  }

  // Export as markdown
  async function exportChatsMarkdown(): Promise<string> {
    try {
      return await invoke<string>('export_chats_markdown')
    } catch (err) {
      console.error('Failed to export markdown:', err)
      return '# 导出失败'
    }
  }

  // Get memory info
  async function getMemoryInfo() {
    try {
      return await invoke<any>('get_memory_info')
    } catch (err) {
      console.error('Failed to get memory info:', err)
      return null
    }
  }

  // Clear all chats
  async function clearAllChats() {
    try {
      await invoke('clear_all_chats')
      messages.value = []
    } catch (err) {
      console.error('Failed to clear all chats:', err)
    }
  }

  return {
    messages,
    isLoading,
    enabled,
    maxMessages,
    loadConfig,
    sendMessage,
    clearMessages,
    loadHistory,
    clearHistory,
    exportAllChats,
    exportChatsMarkdown,
    getMemoryInfo,
    clearAllChats,
  }
})
