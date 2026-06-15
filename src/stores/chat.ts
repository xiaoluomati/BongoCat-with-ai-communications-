import { defineStore } from 'pinia'
import { ref, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import { useTTSStore } from './tts'
import { useConfigStore } from './config'
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
  const configStore = useConfigStore()

  const messages = ref<ChatMessage[]>([])
  const isLoading = ref(false)
  const enabled = ref(false)
  const maxMessages = ref(100)

  // TTS store instance for streaming
  const ttsStore = useTTSStore()
  
  // Emotion parser state for streaming
  let emotionParser: ParserContext = createParserContext()
  
  // Stream listener reference for cleanup
  let unlistenChunkRef: (() => void) | null = null

  // ── Retry queue for failed message saves ──────────────────────────
  // If save_chat_message fails (e.g. broken Tauri bridge, disk error),
  // we retry once then queue. The queue is flushed when the next save
  // succeeds, so no message is permanently lost.
  const pendingSaves: ChatMessage[] = []
  const MAX_RETRY_DELAY = 500

  function sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms))
  }

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

  // Save a single message (with retry)
  async function trySaveOne(message: ChatMessage): Promise<boolean> {
    try {
      await invoke('save_chat_message', {
        characterId: configStore.currentCharacterId,
        message: {
          id: message.id,
          role: message.role,
          content: message.content,
          timestamp: message.timestamp,
        },
      })
      return true
    } catch {
      return false
    }
  }

  // Flush all queued saves; returns ones that still failed
  async function flushPendingSaves(): Promise<ChatMessage[]> {
    const stillFailed: ChatMessage[] = []
    for (const msg of pendingSaves) {
      const ok = await trySaveOne(msg)
      if (!ok) stillFailed.push(msg)
    }
    return stillFailed
  }

  // Save message to memory with retry + queue fallback
  async function saveMessageToMemory(message: ChatMessage) {
    // Attempt 1: immediate
    if (await trySaveOne(message)) {
      // Success — also try to flush any previously queued messages
      const failed = await flushPendingSaves()
      pendingSaves.length = 0
      if (failed.length > 0) {
        pendingSaves.push(...failed)
        console.warn(`[chat] ${failed.length} queued messages still failed to save`)
      }
      return
    }

    // Attempt 2: retry after delay
    await sleep(MAX_RETRY_DELAY)
    if (await trySaveOne(message)) {
      const failed = await flushPendingSaves()
      pendingSaves.length = 0
      if (failed.length > 0) {
        pendingSaves.push(...failed)
        console.warn(`[chat] ${failed.length} queued messages still failed to save`)
      }
      return
    }

    // Both attempts failed — queue for later
    pendingSaves.push(message)
    console.warn(`[chat] message queued for retry (queue size: ${pendingSaves.length})`)
  }
  
  // Build system prompt with emotion tag requirement
  function buildSystemPrompt(basePrompt: string | null, emotionAuto: boolean): string | null {
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

    // Scroll to bottom when user message is added
    nextTick(() => {
      const container = document.querySelector('.messages-container') as HTMLElement
      if (container) container.scrollTop = container.scrollHeight
    })

    // NOTE: user message is NOT saved to file yet.
    // It will only be persisted together with the assistant response
    // after a successful LLM call, keeping the file consistent.

    // Trim messages if needed
    if (messages.value.length > maxMessages.value) {
      messages.value = messages.value.slice(-maxMessages.value)
    }

    try {
      // Read from cached configStore — avoids redundant IPC
      const isStreaming = configStore.llmConfig?.stream ?? false
      const emotionAuto = configStore.ttsConfig?.emotion_auto ?? false

      // Build system prompt with emotion requirement if enabled
      const systemPrompt = buildSystemPrompt(null, emotionAuto)

      if (isStreaming) {
        // Streaming mode: listen for chunks
        const streamingMessageId = `msg_${Date.now() + 1}`
        let streamingContent = ''
        let pureTextContent = ''
        let chunkCount = 0
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

        // Remove any existing listeners first to avoid duplicates
        if (unlistenChunkRef) {
          unlistenChunkRef()
          unlistenChunkRef = null
        }
        
        console.log('[chat] setting up stream listener');
        // Listen for chunks
        unlistenChunkRef = await listen<[string, string]>('chat_stream_chunk', async (event) => {
          const [, chunk] = event.payload
          console.log('[chat] stream chunk received, len=', chunk.length, 'chunk=', JSON.stringify(chunk));
          streamingContent += chunk
          chunkCount++

          if (emotionAuto) {
            const { context, result } = parseEmotionChunk(emotionParser, chunk)
            emotionParser = context

            if (result.emotion) {
              ttsStore.setEmotion(result.emotion)
            }

            // TTS: fire-and-forget — do NOT await, so UI updates are not blocked
            const spokenText = result.text || chunk
            ttsStore.speakStream(spokenText).then(audioPath => {
              if (audioPath) {
                streamingAudioFiles.push({ seq: streamingAudioFiles.length, path: audioPath.replace(/^file:\/\//, ''), text: spokenText })
              }
            })

            // Display: accumulate only result.text (emotion stripped), fall back to raw chunk
            if (result.text) {
              pureTextContent += result.text
            } else {
              pureTextContent += chunk
            }
          } else {
            pureTextContent += chunk
            ttsStore.speakStream(chunk).then(audioPath => {
              if (audioPath) {
                streamingAudioFiles.push({ seq: streamingAudioFiles.length, path: audioPath.replace(/^file:\/\//, ''), text: chunk })
              }
            })
          }
          
          // Update message display with accumulated text
          const msgIndex = messages.value.findIndex(m => m.id === streamingMessageId)
          console.log('[chat] msgIndex:', msgIndex, 'pureTextContent len:', pureTextContent.length);
          if (msgIndex !== -1) {
            messages.value[msgIndex].content = pureTextContent
            console.log('[chat] message content updated, msg len:', messages.value[msgIndex].content.length);
          }
          
          // Scroll every 15 chunks
          if (chunkCount % 15 === 0) {
            const container = document.querySelector('.messages-container') as HTMLElement
            if (container) container.scrollTop = container.scrollHeight
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
          const finalContent = msgIndex !== -1
            ? (emotionAuto ? extractPureText(response.content) : response.content)
            : response.content
          if (msgIndex !== -1) {
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
            
            // Save both messages to memory together — only after success
            await saveMessageToMemory(userMessage)
            await saveMessageToMemory(messages.value[msgIndex])
          }

          return finalContent || response.content
        } finally {
          // Cleanup stream listener to prevent duplicate messages
          if (unlistenChunkRef) {
            unlistenChunkRef()
            unlistenChunkRef = null
          }
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

        // Scroll to bottom after assistant message
        nextTick(() => {
          const container = document.querySelector('.messages-container') as HTMLElement
          if (container) container.scrollTop = container.scrollHeight
        })

        // Save both messages to memory together — only after success
        await saveMessageToMemory(userMessage)
        await saveMessageToMemory(assistantMessage)

        // Trigger TTS with emotion if enabled — collect audio path for replay
        if (emotionAuto && ttsStore.emotionAutoEnabled) {
          const audioPath = await ttsStore.speakWithEmotion(finalContent, ttsStore.getEmotion())
          if (audioPath) {
            const today = new Date().toISOString().split('T')[0]
            assistantMessage.tts_meta = {
              date: today,
              audio_files: [{ seq: 0, path: audioPath.replace(/^file:\/\//, ''), text: finalContent }],
            }
            try {
              await invoke('save_tts_meta', {
                msgId: assistantMessage.id,
                date: today,
                audioFiles: assistantMessage.tts_meta.audio_files,
              })
            } catch (err) {
              console.error('[TTS] save meta error:', err)
            }
          }
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
      const today = new Date().toISOString().split('T')[0]
      const todayChat = await invoke<any>('get_today_chat', { characterId: configStore.currentCharacterId })
      if (todayChat && todayChat.messages) {
        const msgs: ChatMessage[] = []
        for (const msg of todayChat.messages) {
          const mapped: ChatMessage = {
            id: msg.id || `msg_${msg.timestamp}`,
            role: msg.role,
            content: msg.content,
            timestamp: msg.timestamp,
          }
          // Load TTS meta for replay
          if (msg.role === 'assistant') {
            try {
              const meta = await invoke<any>('get_tts_meta', { msgId: mapped.id, date: today })
              if (meta?.audio_files?.length) {
                mapped.tts_meta = { date: today, audio_files: meta.audio_files }
              }
            } catch { /* meta not found, skip */ }
          }
          msgs.push(mapped)
        }
        messages.value = msgs
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
      return await invoke<string>('export_chats_markdown', { characterId: configStore.currentCharacterId })
    } catch (err) {
      console.error('Failed to export markdown:', err)
      return '# 导出失败'
    }
  }

  // Get memory info
  async function getMemoryInfo() {
    try {
      return await invoke<any>('get_memory_info', { characterId: configStore.currentCharacterId })
    } catch (err) {
      console.error('Failed to get memory info:', err)
      return null
    }
  }

  // Retry: remove the last user+assistant pair and resend the user message
  async function retryMessage(assistantMsg: ChatMessage) {
    // Find the preceding user message
    const idx = messages.value.findIndex(m => m.id === assistantMsg.id)
    if (idx <= 0) return

    const userMsg = messages.value[idx - 1]
    if (userMsg.role !== 'user') return

    // Remove from UI
    messages.value.splice(idx - 1, 2)

    // Remove from today's chat file
    try {
      await invoke('remove_messages_by_id', {
        characterId: configStore.currentCharacterId,
        messageIds: [userMsg.id, assistantMsg.id],
      })
    } catch (err) {
      console.error('[chat] retry: failed to remove messages from file:', err)
    }

    // Resend
    await sendMessage(userMsg.content)
  }

  // Clear all chats
  async function clearAllChats() {
    try {
      await invoke('clear_all_chats', { characterId: configStore.currentCharacterId })
      messages.value = []
    } catch (err) {
      console.error('Failed to clear all chats:', err)
    }
  }

  return {
    configStore,
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
    retryMessage,
  }
})
