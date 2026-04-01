# 流式 TTS 优化详细设计文档

> 版本：2.0
> 日期：2026-04-01
> 状态：待实现

---

## 一、需求概述

### 1.1 目标

降低用户听到 AI 首句回复的延迟，实现"边生成边播放"的效果，像人说话一样自然流畅。

### 1.2 当前问题

```
用户发送消息
     ↓
等待 LLM 完全生成（10-30秒）
     ↓
TTS 合成整个回复
     ↓
开始播放
用户等待时间 = LLM生成时间 + TTS合成时间
```

**总延迟：** 可能高达 20-60 秒

### 1.3 优化后

```
用户发送消息
     ↓
LLM 开始流式生成
     ↓
chunk → 累积 → TTS → 播放
chunk → 累积 → TTS → 播放
...
用户听到首句时间 ≈ 1-3 秒
```

---

## 二、整体架构

### 2.1 数据流图

```
┌──────────────┐
│  用户发送消息 │
└──────┬───────┘
       ↓
┌─────────────────────────────────────────────────────────────┐
│                     前端 Chat Store                          │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  chunk buffer (累积文本)                            │    │
│  │  条件：长度>=阈值 OR 遇到句末标点                   │    │
│  └───────────────────────┬─────────────────────────────┘    │
└──────────────────────────┼─────────────────────────────────┘
                           ↓
                    触发 TTS (异步)
                           ↓
┌─────────────────────────────────────────────────────────────┐
│                     后端 send_message_stream                  │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  LLM chat_stream 事件                               │    │
│  │  emit("chat_stream_chunk", chunk)                  │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                           ↓
                    TTS API 调用
                           ↓
┌─────────────────────────────────────────────────────────────┐
│                     IndexTTS Server                              │
│  POST /run/submit_and_refresh                                │
└─────────────────────────────────────────────────────────────┘
                           ↓
                    返回 audio_url
                           ↓
┌─────────────────────────────────────────────────────────────┐
│                     前端 TTS Store                              │
│  排队播放管理器                                           │
│  播完当前 → 自动触发下一个（像人说话一样）                 │
└─────────────────────────────────────────────────────────────┘
                           ↓
                      播放音频
```

### 2.2 排队播放模式

**核心设计：** 像人说话一样，播完一句再播下一句

| 情况 | 处理方式 |
|------|----------|
| 无音频播放中 | 直接播放 |
| 有音频播放中 | **加入播放队列** |
| 当前播放完 | **自动触发播放下一个** |

### 2.3 并行流水线

```
时间线 ──────────────────────────────────────────────────────────→

LLM:     [chunk1][chunk2][chunk3][chunk4][chunk5]...
              ↓       ↓       ↓       ↓       ↓
Buffer:   [你好][你好，][你好，今天][你好，今天天气][...]
              ↓                       ↓
TTS:           [你好]                    [你好，今天天气]
              ↓                       ↓
Audio:    [播放2s][排队]→[播放2s][排队]→[播放3s]
           ──────────────────────────────
                  说完一句接下一句
```

---

## 三、模块设计

### 3.1 前端 Chat Store

**新增状态：**

```typescript
// TTS buffer 状态
const ttsBuffer = ref('')          // TTS 累积文本
const isStreamingTTS = ref(false)  // 流式 TTS 开关
```

**新增方法：**

```typescript
// 累积文本并检查是否触发 TTS
function pushToTTBuffer(chunk: string): boolean {
  ttsBuffer.value += chunk
  
  const len = ttsBuffer.value.length
  const lastChar = chunk.slice(-1)
  const isEndMark = ['。', '！', '？'].includes(lastChar)
  const isLongComma = lastChar === '，' && len >= 10
  const isTooLong = len >= 50  // 最大 buffer 长度强制触发
  
  return isEndMark || isTooLong || (isLongComma && len >= 20)
}

function flushTTBuffer(): string {
  const text = ttsBuffer.value
  ttsBuffer.value = ''
  return text
}
```

### 3.2 前端 TTS Store

**排队播放核心实现：**

```typescript
// 状态
const audioQueue = ref<string[]>([])    // 播放队列
const currentAudio = ref<HTMLAudioElement | null>(null)
const isPlaying = ref(false)

// 入口：接收文本，触发 TTS
async function speakStream(text: string, voiceId?: string): Promise<void> {
  if (!isEnabled.value) return
  
  try {
    const audioUrl = await invoke<string>('tts_speak', {
      text,
      voiceId: voiceId || null
    })
    
    // 加入播放队列
    enqueue(audioUrl)
  } catch (err) {
    console.error('[TTS] speakStream error:', err)
  }
}

// 加入播放队列
function enqueue(audioUrl: string): void {
  if (!isPlaying.value) {
    playNext(audioUrl)
  } else {
    audioQueue.value.push(audioUrl)
  }
}

// 播放下一个
async function playNext(audioUrl: string): Promise<void> {
  isPlaying.value = true
  const audio = new Audio(audioUrl)
  currentAudio.value = audio
  
  audio.onended = () => {
    currentAudio.value = null
    isPlaying.value = false
    
    // 自动播放下一个
    if (audioQueue.value.length > 0) {
      const nextUrl = audioQueue.value.shift()
      playNext(nextUrl!)
    }
  }
  
  audio.onerror = () => {
    console.error('[TTS] audio play error')
    isPlaying.value = false
    currentAudio.value = null
    
    // 继续播放下一个
    if (audioQueue.value.length > 0) {
      const nextUrl = audioQueue.value.shift()
      playNext(nextUrl!)
    }
  }
  
  await audio.play()
}

// 停止并清空队列
function stop(): void {
  if (currentAudio.value) {
    currentAudio.value.pause()
    currentAudio.value = null
  }
  audioQueue.value = []
  isPlaying.value = false
}
```

### 3.3 后端 Chat 命令

**新增命令：**

```rust
/// 流式发送消息（支持 TTS 流式触发）
#[tauri::command]
pub async fn send_message_stream(
    request: ChatRequestInput,
    chat_state: State<'_, Arc<RwLock<ChatState>>>,
    llm_manager: State<'_, Arc<LLMManager>>,
    app_handle: tauri::AppHandle,
) -> Result<ChatResponseOutput, String> {
    // Build context
    let messages = build_full_context(...).await?;
    
    // Add user message
    let user_message = ChatMessage::user(&request.message);
    let mut state = chat_state.write().await;
    state.messages.push(user_message);
    drop(state);
    
    // Stream mode - emit chunks via events
    let chat_id = uuid::Uuid::new_v4().to_string();
    let _ = app_handle.emit("chat_stream_start", (&chat_id,));
    
    let app_handle_clone = app_handle.clone();
    
    let response = llm_manager.chat_stream(messages, move |chunk| {
        let _ = app_handle_clone.emit("chat_stream_chunk", (&chat_id, &chunk));
    }).await.map_err(|e| e.to_string())?;
    
    let _ = app_handle.emit("chat_stream_end", (&chat_id,));
    
    // Save assistant message
    let assistant_message = ChatMessage::assistant(&response.content);
    let mut state = chat_state.write().await;
    state.messages.push(assistant_message);
    
    Ok(response.into())
}
```

---

## 四、触发策略

### 4.1 TTS 触发条件

| 条件 | 说明 | 示例 |
|------|------|------|
| 句末标点 | 遇到句号/问号/感叹号 | "你好。" → 立即触发 |
| 最大长度 | buffer 超过 50 字符 | 防止单次过长 |
| 逗号阈值 | 逗号且长度 >= 20 | "你好，今天天气真不错，" (10字符) 不触发 |

### 4.2 触发参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| triggerThreshold | 20 | 累积超过此长度触发 |
| maxBufferLength | 50 | 超过此长度强制触发 |
| sentenceEndings | [。！？] | 句末标点 |
| commaTriggerLength | 10 | 逗号触发阈值 |

---

## 五、播放队列管理

### 5.1 队列状态

```
队列: [url1, url2, url3]
当前: url1 (播放中)
下一个: url1播完后自动播放url2
```

### 5.2 队列操作

| 操作 | 触发时机 |
|------|----------|
| 入队 | speakStream() 调用时已有音频在播放 |
| 出队 | 当前音频 onended 事件 |
| 清空 | stop() 调用时 |

### 5.3 错误处理

| 错误类型 | 处理方式 |
|----------|----------|
| TTS API 超时 | 静默失败，继续下一个 chunk |
| TTS API 返回错误 | 静默失败，继续下一个 chunk |
| 音频播放失败 | onerror 处理，继续下一个 |
| IndexTTS 未启动 | 静默失败，console.error |

---

## 六、配置设计

### 6.1 流式 TTS 配置

```typescript
interface TTSStreamConfig {
  enabled: boolean           // 流式 TTS 总开关
  triggerThreshold: number   // 触发阈值（默认 20 字符）
  maxBufferLength: number   // 最大 buffer 长度（默认 50）
}
```

### 6.2 配置位置

在 TTS 设置页面添加流式 TTS 开关和参数设置。

---

## 七、文件改动清单

### 后端

| 文件 | 改动 |
|------|------|
| `src-tauri/src/commands/chat.rs` | 新增 `send_message_stream` 命令 |

### 前端

| 文件 | 改动 |
|------|------|
| `src/stores/tts.ts` | 重写为排队播放模式，新增 `speakStream`, `enqueue`, `playNext` |
| `src/stores/chat.ts` | 流式模式下调用 `send_message_stream`，chunk 处理逻辑 |
| `src/pages/comprehensive_function/components/tts/index.vue` | 添加流式 TTS 配置开关 |

---

## 八、实现步骤

### Phase 1: 基础流式

1. [ ] 后端新增 `send_message_stream` 命令
2. [ ] 前端 TTS Store 实现排队播放模式
3. [ ] 前端 Chat Store 集成 chunk 处理逻辑
4. [ ] 配置页面添加流式 TTS 开关

### Phase 2: 优化

1. [ ] 优化触发策略（标点感知）
2. [ ] 添加参数配置界面
3. [ ] 测试与调参

---

## 九、注意事项

### 9.1 兼容性

- 流式 TTS 是**可选项**，可关闭
- 非流式模式保持现有逻辑
- 用户可随时切换

### 9.2 性能考虑

- TTS 调用**异步执行**，不阻塞 chunk 处理
- Buffer 长度限制防止内存积累
- 播放新音频时**不会**停止旧音频，而是加入队列

### 9.3 IndexTTS 限制

- 不支持流式返回
- 每个 TTS 请求是独立调用
- 需注意 API 频率限制

---

## 十、版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 1.0 | 2026-04-01 | 初始设计文档 |
| 2.0 | 2026-04-01 | 添加排队播放模式，说完一句再播下一句 |
