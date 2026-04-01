# TTS 历史记录功能设计文档

> 版本：2.0
> 日期：2026-04-01
> 状态：待实现

---

## 一、设计目标

简化 TTS 历史记录功能：
- **不单独做历史面板**，在聊天面板直接重放
- LLM 回复的消息关联 TTS 音频文件
- 消息旁显示 🔊 图标，点击可重放
- TTS 产物（音频文件）保存到用户目录
- 默认不清除所有产物，除非用户主动删除

---

## 二、存档结构

### 2.1 存档目录

**路径：** `~/.cache/bongo-cat/tts/archive/`

```
~/.cache/bongo-cat/tts/
├── archive/
│   ├── 2026-04-01/
│   │   ├── msg_1711948800_001.wav
│   │   ├── msg_1711948800_002.wav
│   │   ├── msg_1711948800_003.wav
│   │   └── meta.json
│   └── 2026-04-02/
│       └── ...
└── cache/                      # 临时缓存（可清理）
    └── ...
```

### 2.2 元数据文件

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
      { "seq": 2, "path": "msg_1711948800_002.wav", "text": "今天天气真不错" },
      { "seq": 3, "path": "msg_1711948800_003.wav", "text": "。" }
    ],
    "timestamp": 1711948800
  }
}
```

### 2.3 文件命名规则

```
{消息ID}_{序号}.wav
示例：msg_1711948800_001.wav
```

---

## 三、数据流

### 3.1 TTS 播放时保存文件

```
LLM 回复 "你好，今天天气真不错"
     ↓
TTS chunks: ["你好，", "今天天气真不错", "。"]
     ↓
每个 chunk 调用 tts_speak
     ↓
tts_speak 返回 audio_url (可能是 /tmp/xxx.wav)
     ↓
copy 文件到 archive/{date}/{msg_id}_{seq}.wav
     ↓
更新 meta.json
     ↓
返回本地路径给前端播放
```

### 3.2 消息与音频关联

```typescript
interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  timestamp: number
  // 新增
  tts_meta?: {
    date: string           // 日期目录
    audio_files: string[]  // 本地存档路径列表
  }
}
```

### 3.3 重放流程

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

## 四、后端改动

### 4.1 tts_speak 修改

```rust
#[tauri::command]
pub async fn tts_speak(
    text: String,
    voice_id: Option<String>,
    msg_id: Option<String>,  // 新增：关联的消息ID
    seq: Option<u32>,       // 新增：该音频在消息中的序号
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // ... 调用 IndexTTS API ...
    
    // 保存到 archive
    let archive_path = save_to_archive(&audio_bytes, &msg_id, seq)?;
    
    // 返回本地存档路径，而非 API 返回的临时路径
    Ok(archive_path)
}
```

### 4.2 新增命令

```rust
// 获取消息的 TTS 音频列表
#[tauri::command]
pub fn get_tts_meta(msg_id: String, date: String) -> Result<TTSMeta, String>

// 重放消息的 TTS（返回音频路径列表）
#[tauri::command]
pub fn replay_tts(msg_id: String, date: String) -> Result<Vec<String>, String>
```

### 4.3 存档函数

```rust
fn save_to_archive(
    audio_bytes: &[u8],
    msg_id: &Option<String>,
    seq: Option<u32>,
) -> Result<String, String> {
    let archive_dir = dirs::data_local_dir()
        .unwrap()
        .join("cache/bongo-cat/tts/archive")
        .join(get_today_date());
    
    fs::create_dir_all(&archive_dir)?;
    
    let filename = match (msg_id, seq) {
        (Some(id), Some(s)) => format!("{}_{:03}.wav", id, s),
        _ => format!("{}.wav", uuid::Uuid::new_v4()),
    };
    
    let path = archive_dir.join(&filename);
    fs::write(&path, audio_bytes)?;
    
    Ok(path.to_string_lossy().to_string())
}
```

---

## 五、前端改动

### 5.1 ChatMessage 扩展

```typescript
interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  timestamp: number
  // 新增
  tts_meta?: {
    date: string
    audio_files: string[]
  }
}
```

### 5.2 消息显示

在消息组件中，当 `tts_meta` 存在时显示 🔊 图标：

```vue
<template>
  <div class="message" :class="message.role">
    <div class="content">{{ message.content }}</div>
    <div
      v-if="message.tts_meta?.audio_files?.length"
      class="tts-replay"
      @click="replayTTS(message)"
    >
      🔊
    </div>
  </div>
</template>

<script setup>
function replayTTS(message) {
  if (message.tts_meta?.audio_files) {
    message.tts_meta.audio_files.forEach(path => {
      ttsStore.enqueue(path)
    })
  }
}
</script>
```

### 5.3 流式 TTS 修改

```typescript
// 当前消息的 chunk 累积
const currentMsgAudioFiles = ref<string[]>([])

// 收到 chunk 时
async function onChunk(chunk) {
  buffer.value += chunk
  // 触发 TTS
  const audioUrl = await invoke('tts_speak', {
    text: chunk,
    msg_id: currentMsgId,
    seq: currentMsgAudioFiles.value.length + 1
  })
  currentMsgAudioFiles.value.push(audioUrl)
}

// 消息结束时保存 meta
async function onMessageEnd() {
  // 保存 meta 到消息
  await invoke('save_tts_meta', {
    msg_id: currentMsgId,
    date: getTodayDate(),
    audio_files: currentMsgAudioFiles.value
  })
}
```

---

## 六、清理策略

| 类型 | 位置 | 清理方式 |
|------|------|----------|
| 临时缓存 | `~/.cache/bongo-cat/tts/cache/` | 用户可手动清理 |
| 存档文件 | `~/.cache/bongo-cat/tts/archive/` | **默认保留，不自动清理** |
| 用户主动删除 | - | 提供"删除某条消息TTS"按钮 |

---

## 七、文件改动清单

### 后端

| 文件 | 改动 |
|------|------|
| `tts.rs` | `tts_speak` 添加 msg_id/seq，保存到 archive |
| `commands/mod.rs` | 导出新命令 |
| `commands/lib.rs` | 注册新命令 |

### 前端

| 文件 | 改动 |
|------|------|
| `stores/chat.ts` | ChatMessage 扩展 tts_meta |
| `stores/tts.ts` | enqueue 支持本地路径 |
| `components/chat/Message.vue` | 显示 🔊 图标，replayTTS |

---

## 八、版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 1.0 | 2026-04-01 | 初始设计（独立历史面板） |
| 2.0 | 2026-04-01 | 简化设计，在聊天面板内嵌重放 |
