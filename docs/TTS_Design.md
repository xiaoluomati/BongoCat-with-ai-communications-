# TTS 语音合成功能完整实现设计文档

> 版本：3.0
> 日期：2026-04-01
> 状态：待实现

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

### 功能一：音频播放

**实现方案：** 前端使用 HTML5 Audio 播放

**原因：** 
- IndexTTS 返回的是 URL，不是二进制
- 前端播放更简单，不需要后端引入音频库

### 功能二：角色音色绑定

**实现方案：** 
1. 角色配置中添加 `voice_id` 字段
2. 前端角色编辑页面添加音色选择下拉框
3. 后端提供获取当前角色音色的命令

### 功能三：对话中自动触发 TTS

**实现方案：** 后端在 `send_message` 成功后异步触发 TTS

---

## 四、后端实现

### 4.1 tts_speak 命令修改

**返回值：** `audio_url` (String)

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

### 4.2 角色配置添加 voice_id

**config.rs - Character 结构：**
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

### 4.3 新增命令

**character.rs：**
```rust
/// 获取当前角色的音色 ID
#[tauri::command]
pub async fn get_current_character_voice_id() -> Result<Option<String>, String>
```

### 4.4 send_message 触发 TTS

**chat.rs：**
```rust
// 在 send_message 成功后
tauri::async_runtime::spawn(async move {
    let voice_id = get_current_character_voice_id_internal().await;
    let _ = invoke("tts_speak", ("text": response.content, "voice_id": voice_id)).await;
});
```

---

## 五、前端实现

### 5.1 TTS Store

**stores/tts.ts：**
```typescript
export const useTTSStore = defineStore('tts', () => {
  const currentAudio = ref<HTMLAudioElement | null>(null)
  const isPlaying = ref(false)
  
  async function speak(text: string, voiceId?: string): Promise<void> {
    try {
      const audioUrl = await invoke<string>('tts_speak', {
        text,
        voiceId: voiceId || null
      })
      
      stop()
      
      const audio = new Audio(audioUrl)
      currentAudio.value = audio
      isPlaying.value = true
      
      audio.onended = () => {
        isPlaying.value = false
        currentAudio.value = null
      }
      
      await audio.play()
    } catch (err) {
      console.error('TTS error:', err)
    }
  }
  
  function stop() {
    if (currentAudio.value) {
      currentAudio.value.pause()
      currentAudio.value = null
      isPlaying.value = false
    }
  }
  
  return { speak, stop, isPlaying, currentAudio }
})
```

### 5.2 Chat Store 集成

```typescript
// 在收到 AI 回复时
async function onAIResponse(content: string) {
  messages.value.push({ role: 'assistant', content })
  
  const config = await invoke<TTSConfig>('get_tts_config')
  if (config.enabled) {
    const voiceId = await invoke<string | null>('get_current_character_voice_id')
    await ttsStore.speak(content, voiceId || undefined)
  }
}
```

### 5.3 角色编辑页面改动

- 添加音色选择下拉框
- 选项来自 TTS 配置中的 voices 列表

---

## 六、文件改动清单

### 后端

| 文件 | 改动 |
|------|------|
| `src-tauri/src/commands/tts.rs` | 修改 `tts_speak` 返回 URL |
| `src-tauri/src/commands/config.rs` | Character 添加 `voice_id` 字段 |
| `src-tauri/src/commands/chat.rs` | send_message 中添加 TTS 触发 |
| `src-tauri/src/commands/character.rs` | 添加 `get_current_character_voice_id` 命令 |
| `src-tauri/src/commands/mod.rs` | 导出新命令 |
| `src-tauri/src/lib.rs` | 注册新命令 |

### 前端

| 文件 | 改动 |
|------|------|
| `src/stores/tts.ts` | 新增 TTS Store（音频播放） |
| `src/stores/chat.ts` | 集成 TTS 自动播放 |
| `src/pages/comprehensive_function/components/character/index.vue` | 添加音色选择 |
| `src/pages/comprehensive_function/components/tts/index.vue` | 更新（已实现） |

---

## 七、配置结构

### Character 配置

```json
{
  "id": "cat",
  "name": "猫咪",
  "system_prompt": "...",
  "preset_prompt": "...",
  "avatar": "...",
  "preferred_address": "亲爱的",
  "voice_id": "suyao"
}
```

---

## 八、实现清单

### 待完成

- [ ] 后端 tts_speak 修改（返回 URL）
- [ ] 角色配置添加 voice_id
- [ ] 新增 get_current_character_voice_id 命令
- [ ] send_message 触发 TTS
- [ ] 前端 TTS Store
- [ ] Chat Store 集成 TTS
- [ ] 角色编辑页面添加音色选择

### 已完成

- [x] TTS 基础框架
- [x] IndexTTS API 适配
- [x] TTS 设置界面
- [x] 音色管理

---

## 九、版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 1.0 | 2026-03-31 | 初始设计文档 |
| 2.0 | 2026-04-01 | 根据 IndexTTS API 更新（POST 接口） |
| 3.0 | 2026-04-01 | 完整实现设计（音频播放+角色绑定+自动触发） |
