import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import { useTTSStore } from './tts'
import { createParserContext, parseEmotionChunk, getEmotionPrompt, extractPureText, type ParserContext } from '@/utils/emotion'

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
  
  // Emotion parser state for streaming
  let emotionParser: ParserContext = createParserContext()

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
  
  // Build system prompt with emotion tag requirement
  async function buildSystemPrompt(basePrompt: string | null): Promise<string | null> {
    // Check if TTS emotion auto is enabled
    const config = await invoke<any>('load_config')
    const emotionAuto = config.tts?.emotion_auto ?? false
    
    if (!emotionAuto || !basePrompt) {
      return basePrompt
    }
    
    // Append emotion prompt
    return basePrompt + '\n\n' + getEmotionPrompt()
  }

  // Send message
  async function sendMessage(content: string): Promise<string | null> {
    if (!enabled.value || isLoading.value) return null

    isLoading.value = true
    
    // Reset emotion parser for new message
    emotionParser = createParserContext()

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
      // Load config and check settings
      const config = await invoke<any>('load_config')
      const isStreaming = config.llm?.stream ?? false
      const emotionAuto = config.tts?.emotion_auto ?? false
      
      // Build system prompt with emotion requirement if enabled
      const systemPrompt = await buildSystemPrompt(null)

      if (isStreaming) {
        // Streaming mode: listen for chunks
        const streamingMessageId = `msg_${Date.now() + 1}`
        let streamingContent = ''
        let pureTextContent = ''
        let _unlistenChunk: UnlistenFn | null = null
        let _unlistenEnd: UnlistenFn | null = null
        
        // TTS audio tracking
        const streamingAudioFiles: TTSAudioFile[] = []
        const today = new Date().toISOString().split('T')[0]

        // Create placeholder message
        const assistantMessage: ChatMessage = {
          id: streamingMessageId,
          role: 'assistant',
          content: '',
          timestamp: Date.now(),
        }
        messages.value.push(assistantMessage)

        // Listen for chunks and trigger TTS streaming with emotion parsing
        await listen<[string, string]>('chat_stream_chunk', (event) => {
          const [, chunk] = event.payload
          streamingContent += chunk
          
          // Parse emotion from chunk
          if (emotionAuto) {
            const { context, result } = parseEmotionChunk(emotionParser, chunk)
            emotionParser = context
            
            // Update emotion if parsed
            if (result.emotion) {
              ttsStore.setEmotion(result.emotion)
            }
            
            // Append only pure text (no emotion tags) to message
            if (result.text) {
              pureTextContent += result.text
              // Update the message in place with pure text
              const msgIndex = messages.value.findIndex(m => m.id === streamingMessageId)
              if (msgIndex !== -1) {
                messages.value[msgIndex].content = pureTextContent
              }
            }
            
            // Trigger TTS streaming
            ttsStore.speakStream(result.text || chunk)
          } else {
            // No emotion parsing, use raw chunk
            pureTextContent += chunk
            const msgIndex = messages.value.findIndex(m => m.id === streamingMessageId)
            if (msgIndex !== -1) {
              messages.value[msgIndex].content = pureTextContent
            }
            ttsStore.speakStream(chunk)
          }
        })

        // Send message using stream command
        try {
          const response = await invoke<any>('send_message_stream', {
            request: {
              message: content,
              system_prompt: systemPrompt,
            },
          })

          // Final update - ensure pure text
          const msgIndex = messages.value.findIndex(m => m.id === streamingMessageId)
          if (msgIndex !== -1) {
            // Use extracted pure text or response content
            const finalContent = emotionAuto 
              ? extractPureText(response.content)
              : response.content
            messages.value[msgIndex].content = finalContent
            
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
            
            // Save to memory with pure text
            await saveMessageToMemory(messages.value[msgIndex])
          }

          return finalContent || response.content
        } finally {
          // Cleanup listeners would be handled by Tauri automatically
        }
      } else {
        // Non-streaming mode: get full response at once
        const response = await invoke<any>('send_message', {
          request: {
            message: content,
            system_prompt: systemPrompt,
          },
        })

        // Parse emotion from response if enabled
        let finalContent = response.content
        if (emotionAuto) {
          const { result } = parseEmotionChunk(createParserContext(), response.content)
          finalContent = result.text || extractPureText(response.content)
          
          // Set emotion for TTS
          if (result.emotion) {
            ttsStore.setEmotion(result.emotion)
          }
        }

        // Add assistant message with pure text
        const assistantMessage: ChatMessage = {
          id: `msg_${Date.now()}`,
          role: 'assistant',
          content: finalContent,
          timestamp: Date.now(),
        }
        messages.value.push(assistantMessage)

        // Save to memory
        await saveMessageToMemory(assistantMessage)

        // Trigger TTS with emotion if enabled
        if (emotionAuto && ttsStore.emotionAutoEnabled) {
          ttsStore.speakWithEmotion(finalContent, ttsStore.getEmotion())
        }

        // Trim again
        if (messages.value.length > maxMessages.value) {
          messages.value = messages.value.slice(-maxMessages.value)
        }

        return finalContent
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
