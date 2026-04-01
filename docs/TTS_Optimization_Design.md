# TTS 功能优化设计文档

> 版本：1.0
> 日期：2026-04-01
> 状态：待实现

---

## 一、优化概览

| 编号 | 功能 | 优先级 | 说明 |
|------|------|--------|------|
| 1 | 流式 TTS 配置开关 | 🔴 高 | 界面开关控制开启/关闭 |
| 2 | 触发策略参数可配置 | 🔴 高 | 可调整阈值参数 |
| 3 | 播放控制 | 🟡 中 | 暂停/继续/跳过/停止 |
| 4 | 音频渐入渐出 | 🟡 中 | 避免突然开始/结束 |
| 8 | 播放历史记录 | 🟢 低 | 回听历史 TTS |

---

## 二、流式 TTS 配置开关

### 2.1 需求

当前流式 TTS 没有界面开关，用户无法直观控制开启/关闭。

### 2.2 设计

**新增配置项：**

```typescript
interface TTSConfig {
  // 现有字段...
  stream_enabled: boolean  // 流式 TTS 开关
}
```

**界面设计：**

```
┌─────────────────────────────────────┐
│ 启用 TTS  [●─────────────────────] │ ← 总开关
├─────────────────────────────────────┤
│ 流式模式   [●─────────────────────] │ ← 新增：流式开关
├─────────────────────────────────────┤
│ 服务地址   [http://localhost:9880]  │
│ 音量       [──────●──────] 80      │
│ ...                                   │
└─────────────────────────────────────┘
```

**代码改动：**

| 文件 | 改动 |
|------|------|
| `config.rs` | TTSConfig 添加 `stream_enabled` 字段 |
| `tts.rs` | `get_current_character_voice_id_internal` 返回流式开关状态 |
| 前端 TTS Store | 根据 `stream_enabled` 决定使用 `speak` 或 `speakStream` |
| `index.vue` | 添加流式模式开关 UI |

---

## 三、触发策略参数可配置

### 3.1 需求

当前触发阈值硬编码，无法根据实际效果调整。

### 3.2 可配置参数

| 参数 | 当前值 | 可调范围 | 说明 |
|------|--------|----------|------|
| triggerThreshold | 20 | 10-50 | 触发阈值（字符数） |
| maxBufferLength | 50 | 30-100 | 最大缓冲长度 |
| minChunkLength | 5 | 3-10 | 最小 chunk 长度 |

### 3.3 界面设计

```
┌─────────────────────────────────────┐
│ 流式参数配置                        │
├─────────────────────────────────────┤
│ 触发阈值   [──────●──────] 20 字符  │
│ 最大缓冲   [──────●──────] 50 字符  │
│ 最小触发   [──────●──────] 5 字符  │
└─────────────────────────────────────┘
```

**提示文字：**
- 触发阈值：累积超过此值时触发 TTS（句子较短时可设低）
- 最大缓冲：超过此值强制触发（防止单次过长）
- 最小触发：必须达到此长度才触发（避免太短的碎片）

### 3.4 代码改动

```typescript
// TTSStore 新增配置
interface TTSConfig {
  // 现有字段...
  stream_enabled: boolean
  stream_trigger_threshold: number  // 20
  stream_max_buffer: number       // 50
  stream_min_chunk: number         // 5
}
```

---

## 四、播放控制

### 4.1 需求

用户需要控制 TTS 播放行为：暂停、继续、跳过、停止。

### 4.2 控制功能

| 功能 | 说明 |
|------|------|
| 暂停 | 暂停当前播放，保留队列 |
| 继续 | 从暂停位置继续播放 |
| 跳过 | 停止当前，播放下一个 |
| 停止 | 清空队列，停止播放 |

### 4.3 界面设计

**方案 A：全局控制按钮**

```
┌─────────────────────────────────────┐
│ 🔊 TTS: 正在播放... [⏸暂停][⏭跳过][⏹停止] │
└─────────────────────────────────────┘
```

**方案 B：悬浮控制条**

当有 TTS 播放时，底部显示悬浮控制条：
- 显示当前播放的文本片段
- 播放/暂停按钮
- 停止按钮

### 4.4 代码改动

```typescript
// TTSStore 新增方法
function pause(): void {
  if (currentAudio.value) {
    currentAudio.value.pause()
    isPaused.value = true
  }
}

function resume(): void {
  if (currentAudio.value && isPaused.value) {
    currentAudio.value.play()
    isPaused.value = false
  }
}

function skip(): void {
  if (currentAudio.value) {
    currentAudio.value.pause()
    // 触发 onended 逻辑，自动播放下一个
    currentAudio.value = null
  }
}
```

---

## 五、音频渐入渐出

### 5.1 需求

当前音频突然开始/结束，用户体验不佳。

### 5.2 设计

| 效果 | 时长 | 说明 |
|------|------|------|
| 渐入 | 200ms | 音量从 0 → 目标音量 |
| 渐出 | 200ms | 目标音量 → 0，然后停止 |

### 5.3 代码实现

```typescript
const FADE_DURATION = 200  // ms
const FADE_STEPS = 10

async function fadeIn(audio: HTMLAudioElement, targetVolume: number): Promise<void> {
  audio.volume = 0
  await audio.play()
  
  for (let i = 1; i <= FADE_STEPS; i++) {
    audio.volume = (targetVolume / 100) * (i / FADE_STEPS)
    await sleep(FADE_DURATION / FADE_STEPS)
  }
}

async function fadeOut(audio: HTMLAudioElement): Promise<void> {
  const startVolume = audio.volume
  
  for (let i = FADE_STEPS; i >= 0; i--) {
    audio.volume = startVolume * (i / FADE_STEPS)
    await sleep(FADE_DURATION / FADE_STEPS)
  }
  
  audio.pause()
}

// 修改 playNext
async function playNext(audioUrl: string): Promise<void> {
  // ... 渐出上一个
  if (currentAudio.value) {
    await fadeOut(currentAudio.value)
  }
  
  // ... 创建新音频
  const audio = new Audio(audioUrl)
  await fadeIn(audio, volume.value)  // 渐入
}
```

### 5.4 参数可配置

```typescript
interface TTSConfig {
  // 现有字段...
  fade_duration: number  // 渐变时长(ms)，默认 200
}
```

---

## 六、播放历史记录

### 6.1 需求

记录 TTS 播放历史，方便用户回听 AI 之前的回复。

### 6.2 设计

**历史记录结构：**

```typescript
interface TTSHistoryItem {
  id: string
  text: string           // 播放的文本
  voice_id: string       // 使用的音色
  timestamp: number       // 播放时间
  audio_url?: string     // 缓存的音频 URL（可选）
}
```

**存储位置：** `~/.cache/bongo-cat/tts/history.json`

**界面设计：**

```
┌─────────────────────────────────────┐
│ TTS 播放历史                    [清空] │
├─────────────────────────────────────┤
│ 🔊 14:32 "你好，今天天气真不错..."  苏瑶 │
│ 🔊 14:30 "很高兴见到你！"          苏瑶 │
│ 🔊 14:28 "喵~"                    凯茜娅 │
└─────────────────────────────────────┘
```

**点击播放：** 点击历史记录可重新播放该 TTS。

**代码改动：**

```typescript
// TTSStore
const history = ref<TTSHistoryItem[]>([])
const MAX_HISTORY = 50  // 最多保存 50 条

function addToHistory(text: string, voiceId: string): void {
  history.value.unshift({
    id: `tts_${Date.now()}`,
    text,
    voice_id: voiceId,
    timestamp: Date.now()
  })
  
  // 超过上限则删除最老的
  if (history.value.length > MAX_HISTORY) {
    history.value.pop()
  }
  
  saveHistoryToFile()
}

function playFromHistory(item: TTSHistoryItem): void {
  // 重新调用 tts_speak 并播放
  speak(item.text, item.voice_id)
}
```

---

## 七、文件改动清单

### 后端

| 文件 | 改动 |
|------|------|
| `config.rs` | TTSConfig 添加新配置字段 |
| `tts.rs` | 提供历史记录接口 |

### 前端

| 文件 | 改动 |
|------|------|
| `stores/tts.ts` | 流式开关、播放控制、渐变、历史记录 |
| `pages/tts/index.vue` | 流式开关 UI、参数调节 UI、控制按钮、历史面板 |

---

## 八、实现步骤

### Phase 1: 配置基础

1. [ ] TTSConfig 添加 `stream_enabled`
2. [ ] TTSStore 集成 `stream_enabled` 逻辑
3. [ ] 前端添加流式开关 UI

### Phase 2: 参数可配置

4. [ ] TTSConfig 添加阈值参数
5. [ ] 前端添加参数调节 UI

### Phase 3: 播放控制

6. [ ] TTSStore 添加 `pause/resume/skip/stop`
7. [ ] 前端添加控制按钮 UI

### Phase 4: 渐变效果

8. [ ] TTSStore 添加 `fadeIn/fadeOut`
9. [ ] 配置项 `fade_duration`

### Phase 5: 历史记录

10. [ ] 后端添加历史记录存储
11. [ ] 前端添加历史面板 UI

---

## 九、版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 1.0 | 2026-04-01 | 初始优化设计文档 |
