# TTS 语音合成功能完整设计文档

> 版本：4.0
> 日期：2026-04-02
> 状态：待整合实现

---

## 目录

1. [整体架构](./#一整体架构)
2. [IndexTTS API 规格](./#二indextts-api-规格)
3. [功能拆分](./#三功能拆分)
4. [流式 TTS](./#四流式-tts)
5. [情感识别自动匹配](./#五情感识别自动匹配)
6. [历史记录功能](./#六历史记录功能)
7. [优化功能](./#七优化功能)
8. [后端实现](./#八后端实现)
9. [前端实现](./#九前端实现)
10. [文件改动清单](./#十文件改动清单)
11. [版本历史](./#十一版本历史)

---

## 一、整体架构

### 1.1 流程图

```
用户发送消息
     ↓
前端 invoke('send_message')
     ↓
后端 send_message()
     ↓
┌────────────────────────────────────────┐
│  1. 调用 LLM 获取回复                    │
│  2. 保存消息到 ChatState                 │
│  3. 获取当前角色的 voice_id             │
│  4. 异步调用 tts_speak()               │
└────────────────────────────────────────┘
              ↓ (异步，不阻塞对话)
tts_speak() 执行
     ↓
┌────────────────────────────────────────┐
│  1. 获取 TTS 配置和音色设置             │
│  2. 生成缓存 key (MD5)                │
│  3. 检查缓存是否存在                   │
│     ├─ 存在 → 获取 URL                 │
│     └─ 不存在 → POST /submit_and_refresh │
│              → 解析响应获取 audio_url   │
│              → 保存到缓存               │
└────────────────────────────────────────┘
              ↓
┌────────────────────────────────────────┐
│  返回 audio_url 给前端                 │
│  前端使用 HTML5 Audio 播放            │
└────────────────────────────────────────┘
```

### 1.2 流式模式架构

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

## 二、IndexTTS API 规格

### 2.1 提交合成 `POST /run/submit_and_refresh`

**请求体：**
```json
{
  "voices_dropdown": "苏瑶",
  "speed": 1.0,
  "text": "要合成的文本",
  "emo_control_method": "使用情感描述文本控制",
  "emo_weight": 0.8,
  "emo_text": "高兴",
  "emo_random": false,
  "max_tokens": 100
}
```

**响应：**
```json
{
  "output_1": {
    "path": "/path/to/audio.wav",
    "url": null,
    "_type": "gradio.FileData"
  }
}
```

### 2.2 音频 URL 构造

由于 `url` 通常为 `null`，需要构造文件访问 URL：
```
http://localhost:9880/file=/path/to/audio.wav
```

---

## 三、功能拆分

| 功能 | 说明 | 优先级 |
|------|------|--------|
| 音频播放 | 前端 HTML5 Audio 播放 | 🔴 必需 |
| 角色音色绑定 | 角色配置 voice_id | 🔴 必需 |
| 对话自动触发 TTS | send_message 成功后触发 | 🔴 必需 |
| 流式 TTS | 边生成边播放 | 🔴 高 |
| 情感识别自动匹配 | LLM 回复带情感标签 | 🔴 高 |
| 历史记录 | 消息关联 TTS 音频 | 🟡 中 |
| 流式配置开关 | 界面控制开/关 | 🔴 高 |
| 触发策略参数可配置 | 可调整阈值 | 🔴 高 |
| 播放控制 | 暂停/继续/跳过/停止 | 🟡 中 |
| 音频渐入渐出 | 提升体验 | 🟡 中 |

---

## 四、流式 TTS

### 4.1 数据流图

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

### 4.2 排队播放模式

**核心设计：** 像人说话一样，播完一句再播下一句

| 情况 | 处理方式 |
|------|----------|
| 无音频播放中 | 直接播放 |
| 有音频播放中 | **加入播放队列** |
| 当前播放完 | **自动触发播放下一个** |

### 4.3 触发策略

| 条件 | 说明 | 示例 |
|------|------|------|
| 句末标点 | 遇到句号/问号/感叹号 | "你好。" → 立即触发 |
| 最大长度 | buffer 超过 50 字符 | 防止单次过长 |
| 逗号阈值 | 逗号且长度 >= 20 | "你好，今天天气真不错，" (10字符) 不触发 |

| 参数 | 默认值 | 可调范围 | 说明 |
|------|--------|----------|------|
| triggerThreshold | 20 | 10-50 | 触发阈值（字符数） |
| maxBufferLength | 50 | 30-100 | 最大缓冲长度 |
| minChunkLength | 5 | 3-10 | 最小 chunk 长度 |

---

## 五、情感识别自动匹配

### 5.1 工作流程

```
LLM 回复格式：
<emo>情感</emo>实际回复内容

示例：
<emo>高兴</emo>今天天气真不错！
```

### 5.2 6个情感

| 情感 | 音频文件 |
|------|----------|
| 高兴 | 高兴.wav |
| 伤心 | 伤心.wav |
| 生气 | 生气.wav |
| 害怕 | 害怕.wav |
| 平静 | 平静.wav |
| 嫌弃 | 嫌弃.wav |

### 5.3 状态机解析

```
状态转移：

     ┌─────────┐  '<'   ┌───────────┐  'emo'   ┌────────────┐  '>'   ┌─────────┐  其他字符   ┌─────────┐
TEXT │────────→│ TAG_OPEN │────────→│TAG_VERIFY│────────→│CONTENT │────────→│ TEXT    │
     └─────────┘         └───────────┘           └────────────┘         └─────────┘
                              ↑                                                      │
                              │  不是 'emo'                                            │
                              └───────────────────────────────────────────────────────┘

     ┌─────────┐  '<'   ┌───────────┐  '/'   ┌───────────┐  '>'   ┌─────────┐
TAG_CLOSE │────────→│ SLASH1 │────────→│SLASH2 │────────→│ CHECK │──成功──→│ READY  │
         └─────────┘         └───────────┘         └───────────┘         └─────────┘
```

### 5.4 解析状态

| 状态 | 说明 |
|------|------|
| `TEXT` | 普通文本模式，累积正文 |
| `TAG_OPEN` | 刚遇到 `<`，等待 `emo>` |
| `TAG_VERIFY` | 验证是否为 `<emo>` |
| `CONTENT` | 标签内容模式，提取情感 |
| `TAG_CLOSE` | 遇到 `</`，等待 `emo>` |
| `READY` | 标签解析完成，提取情感成功 |

### 5.5 情感映射表

```typescript
const EMOTION_MAP: Record<string, string> = {
  '高兴': '高兴.wav',
  '开心': '高兴.wav',      // 别名
  '伤心': '伤心.wav',
  '难过': '伤心.wav',      // 别名
  '生气': '生气.wav',
  '愤怒': '生气.wav',      // 别名
  '害怕': '害怕.wav',
  '恐惧': '害怕.wav',      // 别名
  '平静': '平静.wav',
  '淡定': '平静.wav',      // 别名
  '嫌弃': '嫌弃.wav',
  '厌恶': '嫌弃.wav',      // 别名
}

const DEFAULT_EMOTION = '高兴.wav'
```

### 5.6 解析失败处理

| 情况 | 处理 |
|------|------|
| 标签不在已知情感中 | 默认"高兴" |
| 标签为空 | 默认"高兴" |
| 格式错误 | 忽略标签，当普通文本 |
| 情感含别名 | 归一化到主情感 |

### 5.7 提示词设计

```
你的所有回复需要在开头包含情感标签，格式如下：
<emo>情感</emo>

可选情感：高兴、伤心、生气、害怕、平静、嫌弃

注意：
1. 标签必须在回复最开头
2. 标签单独作为一个chunk输出，不要与其他内容混合
3. 不在6个中或空 → 默认"高兴"
```

### 5.8 流式处理时序

```
时间  LLM 输出          前端处理              状态
─────────────────────────────────────────────────
T1    "<"             追加到 buffer         TEXT
T2    "<emo"           追加到 buffer         TAG_OPEN
T3    ">"              追加到 buffer         TAG_VERIFY ✓
T4    "高兴"           追加到 buffer         CONTENT
T5    "</emo>"         提取"高兴"            TAG_CLOSE ✓
T6    "今"             触发 TTS + 显示       TEXT
T7    "天"             触发 TTS + 显示
...
```

### 5.9 聊天记录不记录情感标签

```
LLM 输出：<emo>高兴</emo>今天天气真不错！

存储/显示：今天天气真不错！✅
TTS：高兴.wav ✅
```

---

## 六、历史记录功能

### 6.1 设计目标

- **不单独做历史面板**，在聊天面板直接重放
- LLM 回复的消息关联 TTS 音频文件
- 消息旁显示 🔊 图标，点击可重放
- TTS 产物（音频文件）保存到用户目录
- 默认不清除所有产物，除非用户主动删除

### 6.2 存档结构

**路径：** `~/.cache/bongo-cat/tts/archive/`

```
~/.cache/bongo-cat/tts/
├── archive/
│   ├── 2026-04-01/
│   │   ├── msg_1711948800_001.wav
│   │   ├── msg_1711948800_002.wav
│   │   └── meta.json
│   └── 2026-04-02/
│       └── ...
└── cache/                      # 临时缓存（可清理）
    └── ...
```

### 6.3 元数据文件

```json
// archive/2026-04-01/meta.json
{
  "msg_1711948800": {
    "id": "msg_1711948800",
    "date": "2026-04-01",
    "text": "你好，今天天气真不错",
    "voice_id": "suyao",
    "audio_files": [
      { "seq": 1, "path": "msg_1711948800_001.wav", "text": "你好，" },
      { "seq": 2, "path": "msg_1711948800_002.wav", "text": "今天天气真不错" }
    ],
    "timestamp": 1711948800
  }
}
```

### 6.4 重放流程

```
用户点击消息旁的 🔊 图标
     ↓
查找该消息的 tts_meta.audio_files
     ↓
按顺序 enqueue 到播放队列
     ↓
排队播放（播完一句接下一句）
```

---

## 七、优化功能

### 7.1 流式 TTS 配置开关

```
┌─────────────────────────────────────┐
│ 启用 TTS  [●─────────────────────] │
├─────────────────────────────────────┤
│ 流式模式   [●─────────────────────] │
├─────────────────────────────────────┤
│ 服务地址   [http://localhost:9880]  │
│ 音量       [──────●──────] 80      │
└─────────────────────────────────────┘
```

### 7.2 播放控制

| 功能 | 说明 |
|------|------|
| 暂停 | 暂停当前播放，保留队列 |
| 继续 | 从暂停位置继续播放 |
| 跳过 | 停止当前，播放下一个 |
| 停止 | 清空队列，停止播放 |

### 7.3 音频渐入渐出

| 效果 | 时长 | 说明 |
|------|------|------|
| 渐入 | 200ms | 音量从 0 → 目标音量 |
| 渐出 | 200ms | 目标音量 → 0，然后停止 |

---

## 八、后端实现

### 8.1 tts_speak 命令

```rust
#[tauri::command]
pub async fn tts_speak(
    text: String,
    voice_id: Option<String>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // ... 获取配置和音色 ...
    
    // 调用 IndexTTS API
    let response = client.post(&url).json(&request_body).send().await?;
    let result: SubmitResponse = response.json().await?;
    
    // 解析音频 URL
    let audio_url = result.output?
        .output_1?
        .url
        .or(Some(format!("http://localhost:9880/file={}", 
            result.output?.output_1?.path.unwrap_or_default())));
    
    Ok(audio_url.unwrap_or_default())
}
```

### 8.2 角色配置添加 voice_id

```rust
pub struct Character {
    pub id: String,
    pub name: String,
    pub description: String,
    pub preset_prompt: String,
    pub system_prompt: String,
    pub avatar: String,
    pub preferred_address: String,
    pub voice_id: Option<String>,  // 新增
}
```

### 8.3 send_message_stream 命令

```rust
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

## 九、前端实现

### 9.1 TTS Store 结构

```typescript
export const useTTSStore = defineStore('tts', () => {
  // State
  const currentAudio = ref<HTMLAudioElement | null>(null)
  const audioQueue = ref<string[]>([])
  const isPlaying = ref(false)
  const isPaused = ref(false)
  const isEnabled = ref(false)
  const isStreamMode = ref(false)
  const config = ref<TTSConfig | null>(null)
  
  // TTS buffer for streaming
  const ttsBuffer = ref('')
  
  // 排队播放核心实现
  async function speakStream(text: string, voiceId?: string): Promise<void> {
    if (!isEnabled.value) return
    
    try {
      const audioUrl = await invoke<string>('tts_speak', {
        text,
        voiceId: voiceId || null
      })
      
      enqueue(audioUrl)
    } catch (err) {
      console.error('[TTS] speakStream error:', err)
    }
  }
  
  // 使用指定情感播放
  async function speakWithEmotion(
    text: string, 
    emotion: string, 
    voiceId?: string
  ): Promise<void> {
    // 根据情感选择音频文件
    const audioFile = EMOTION_MAP[emotion] || DEFAULT_EMOTION
    // 调用 TTS
    // ...
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
  
  return { 
    speakStream, 
    speakWithEmotion,
    enqueue, 
    playNext, 
    stop,
    currentAudio,
    isPlaying,
    isPaused,
    isEnabled,
    isStreamMode
  }
})
```

### 9.2 Chat Store 集成

```typescript
// 关键集成逻辑：
// 1. 构建 system_prompt 时追加情感提示词（如果 emotion_auto 开启）
// 2. 流式处理时调用 parseEmotionChunk 解析情感
// 3. 消息 content 只追加纯文本（extractPureText）
// 4. 调用 ttsStore.speakWithEmotion 时传入解析出的情感
```

### 9.3 情感解析核心函数

```typescript
// src/utils/emotion.ts

export enum ParseState {
  TEXT = 'TEXT',
  TAG_OPEN = 'TAG_OPEN',
  TAG_VERIFY = 'TAG_VERIFY', 
  CONTENT = 'CONTENT',
  TAG_CLOSE = 'TAG_CLOSE',
  READY = 'READY',
}

export interface ParseResult {
  emotion: string      // 解析出的情感（用于TTS）
  text: string         // 累积的正文（无标签，用于显示）
  ready: boolean       // 是否解析完成
}

export interface ParserContext {
  state: ParseState
  buffer: string
  tagBuffer: string
  emotion: string
  text: string
}

export function createParserContext(): ParserContext
export function parseEmotionChunk(ctx, chunk): { context; result }
export function parseEmotion(text): ParseResult
export function extractPureText(text): string
export function getEmotionPrompt(): string
```

---

## 十、文件改动清单

### 10.1 新建文件

| 文件 | 说明 |
|------|------|
| `src/utils/emotion.ts` | 情感解析状态机模块 |

### 10.2 后端文件

| 文件 | 改动 |
|------|------|
| `src-tauri/src/commands/tts.rs` | tts_speak, tts_speak_with_emotion |
| `src-tauri/src/commands/config.rs` | TTSConfig 添加 emotion_auto, stream_enabled 等 |
| `src-tauri/src/commands/chat.rs` | send_message_stream 命令 |
| `src-tauri/src/commands/character.rs` | get_current_character_voice_id |
| `src-tauri/src/commands/mod.rs` | 导出新命令 |
| `src-tauri/src/lib.rs` | 注册新命令 |

### 10.3 前端文件

| 文件 | 改动 |
|------|------|
| `src/stores/tts.ts` | 排队播放、speakWithEmotion、流式开关 |
| `src/stores/chat.ts` | 集成情感解析、流式处理 |
| `src/pages/comprehensive_function/components/tts/index.vue` | 配置界面 |
| `src/pages/comprehensive_function/components/character/index.vue` | 音色选择 |

---

## 十一、版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 1.0 | 2026-03-31 | 初始设计文档 |
| 2.0 | 2026-04-01 | 根据 IndexTTS API 更新 |
| 3.0 | 2026-04-01 | 完整实现设计（音频播放+角色绑定+自动触发） |
| 4.0 | 2026-04-02 | 整合所有 TTS 相关文档（流式、情感、历史、优化） |

---

## 十二、待整合实现清单

### Phase 1: 基础功能

- [ ] 后端 tts_speak 修改（返回 URL）
- [ ] 角色配置添加 voice_id
- [ ] 新增 get_current_character_voice_id 命令
- [ ] send_message 触发 TTS
- [ ] 前端 TTS Store
- [ ] Chat Store 集成 TTS
- [ ] 角色编辑页面添加音色选择

### Phase 2: 流式 TTS

- [ ] 后端新增 send_message_stream 命令
- [ ] 前端 TTS Store 实现排队播放模式
- [ ] 前端 Chat Store 集成 chunk 处理逻辑
- [ ] 配置页面添加流式 TTS 开关

### Phase 3: 情感识别

- [ ] 新建 src/utils/emotion.ts
- [ ] Chat Store 集成情感解析
- [ ] TTS Store 新增 speakWithEmotion
- [ ] TTSConfig 添加 emotion_auto
- [ ] 提示词自动追加

### Phase 4: 优化功能

- [ ] 流式参数可配置（triggerThreshold, maxBufferLength, minChunkLength）
- [ ] 播放控制（暂停/继续/跳过/停止）
- [ ] 音频渐入渐出
- [ ] 历史记录面板
