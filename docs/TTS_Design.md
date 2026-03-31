# TTS 语音合成功能设计文档

> 版本：1.0
> 日期：2026-03-31
> 状态：待实现

---

## 一、需求概述

**目标：** 将 AI 对话内容转换为语音输出

**使用场景：** 桌宠回复时，用语音朗读 AI 的回答

**对话流程：**
```
用户发送消息 → AI 生成回复 → 回复保存到聊天 → TTS 播放语音
                         ↑
                         └──────────────────────┘
                         （回复完成后触发 TTS，异步进行）
```

---

## 二、技术方案

### 2.1 IndePTTS2 接口规格

**服务端地址：** `http://localhost:9880`（默认，可配置）

**HTTP Method:** GET

**请求格式：**
```
http://localhost:9880/?text={文本}&speaker={音色}&emo={情感}&weight={权重}
```

**参数说明：**

| 参数 | 类型 | 必填 | 说明 | 示例 |
|------|------|------|------|------|
| `text` | string | ✅ | 要转换的文本 | `你好，今天天气真不错` |
| `speaker` | string | ✅ | 音色名称 | `苏瑶` |
| `emo` | string | ❌ | 情感标签或情感参考文件路径 | `开心` 或 `情感参考/愤怒.wav` |
| `weight` | float | ❌ | 情感强度 (0.0-1.0) | `1.0` |

**返回值：** WAV 音频文件（二进制流，一次性返回）

### 2.2 调用流程

```
用户输入消息
     ↓
点击发送 / 按回车
     ↓
前端调用后端 send_message
     ↓
后端调用 LLM 获取回复
     ↓
回复存入 ChatState
     ↓
触发 TTS（异步，不阻塞对话）
     ↓
根据当前角色获取 voice_id
     ↓
查找对应音色配置
     ↓
调用 IndePTTS2 API
     ↓
获取 WAV 音频数据
     ↓
保存到缓存目录
     ↓
播放音频
```

**关键点：**
- TTS 调用是**异步**的，不影响正常对话流程
- 用户发送消息后，立即显示 AI 回复，无需等待 TTS 播放完成
- 新的 AI 回复时，如果前一个还在播放，则停止前一个

---

## 三、配置设计

### 3.1 音色注册表方案

**核心设计：**
- 音色配置集中存储在 `tts.voices` 下
- 每个角色通过 `voice_id` 引用音色
- 无 `voice_id` 的角色使用全局默认音色（`tts.default_voice_id`）

### 3.2 配置结构

```json
{
  "tts": {
    "enabled": false,
    "base_url": "http://localhost:9880",
    "default_voice_id": "suyao",
    "volume": 80,
    "speed": 1.0,
    "voices": {
      "suyao": {
        "speaker": "苏瑶",
        "emo": "neutral",
        "weight": 1.0
      },
      "xiaoyu": {
        "speaker": "小宇",
        "emo": "happy",
        "weight": 0.9
      },
      "dog_voice": {
        "speaker": "旺财",
        "emo": "energetic",
        "weight": 1.0
      }
    }
  },
  "characters": {
    "current": "cat",
    "cat": {
      "voice_id": "suyao"
    },
    "dog": {
      "voice_id": "dog_voice"
    },
    "robot": {
      "voice_id": null
    }
  }
}
```

### 3.3 配置项说明

**TTS 全局配置：**

| 配置项 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| `enabled` | bool | `false` | TTS 功能总开关 |
| `base_url` | string | `http://localhost:9880` | IndePTTS2 服务地址 |
| `default_voice_id` | string | `suyao` | 默认音色 ID |
| `volume` | int | `80` | 音量 (0~100) |
| `speed` | float | `1.0` | 语速 (0.5~2.0) |

**音色配置 (voices)：**

| 配置项 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| `speaker` | string | - | IndePTTS2 音色名称 |
| `emo` | string | `neutral` | 情感标签或文件路径 |
| `weight` | float | `1.0` | 情感强度 (0.0~1.0) |

**角色配置：**

| 配置项 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| `voice_id` | string | null | 引用的音色 ID，为 null 时使用默认音色 |

---

## 四、缓存设计

### 4.1 缓存策略

使用 `text` + `speaker` + `emo` + `weight` 的组合生成缓存 key（MD5）

### 4.2 缓存位置

系统缓存目录：
- Linux: `~/.cache/bongo-cat/tts/`
- macOS: `~/Library/Caches/bongo-cat/tts/`
- Windows: `%LOCALAPPDATA%\bongo-cat\tts\`

### 4.3 缓存格式

```
tts/
├── a1b2c3d4e5f6g7h8.wav   # 缓存的音频文件
├── 9876543210abcdef.wav
└── ...
```

### 4.4 缓存命中逻辑

```
请求 text="你好" speaker="苏瑶" emo="开心" weight=1.0
     ↓
生成 key = MD5("你好|苏瑶|开心|1.0")
     ↓
检查缓存目录是否存在 key.wav
     ↓
存在 → 直接播放缓存文件
不存在 → 调用 API → 保存到缓存 → 播放
```

### 4.5 缓存清理

不主动清理缓存，用户可手动清理缓存目录

---

## 五、错误处理

| 错误类型 | 处理方式 |
|----------|----------|
| 服务未启动 (connection refused) | 静默失败，记录 debug 日志 |
| 请求超时 | 静默失败，记录 debug 日志 |
| 返回非 200 | 静默失败，记录 error 日志 |
| 音频解码失败 | 静默失败，记录 error 日志 |
| text 为空 | 不调用 API，直接返回 |
| voice_id 不存在 | 使用默认音色 |

**原则：** TTS 失败不影响对话功能，用户无感知

---

## 六、前端界面设计

### 6.1 TTS 设置页面

**入口：** 设置 → 综合功能 → 语音设置（新建 Tab）

### 6.2 UI 元素

| 元素 | 类型 | 说明 |
|------|------|------|
| 启用 TTS 开关 | Switch | 总开关 |
| 服务地址 | Input | `http://localhost:9880` |
| 测试按钮 | Button | 点击朗读示例文本 |

### 6.3 音色管理

| 元素 | 类型 | 说明 |
|------|------|------|
| 音色列表 | List | 显示所有已配置的音色 |
| 添加音色 | Button | 打开添加音色弹窗 |
| 编辑音色 | Button | 打开编辑音色弹窗 |
| 删除音色 | Button | 删除音色（需确认） |

### 6.4 音色配置弹窗

| 元素 | 类型 | 说明 |
|------|------|------|
| 音色 ID | Input | 唯一标识符，如 `suyao` |
| 音色名称 | Input | IndePTTS2 音色名，如 `苏瑶` |
| 情感标签/路径 | Input | 如 `开心` 或 `情感参考/愤怒.wav` |
| 情感强度 | Slider | 0.0 ~ 1.0，默认 1.0 |

### 6.5 角色音色绑定

**入口：** 角色设置页面

| 元素 | 类型 | 说明 |
|------|------|------|
| 音色选择 | Select | 选择已配置的音色，或选择"默认" |

---

## 七、文件结构

### 7.1 后端新增文件

```
src-tauri/src/
├── commands/
│   ├── mod.rs          # 导出 tts 命令
│   └── tts.rs          # TTS 命令实现
```

### 7.2 前端新增/修改文件

```
src/pages/comprehensive_function/components/
├── tts/                # 新建 TTS 设置组件
│   ├── index.vue       # 主组件
│   ├── VoiceList.vue   # 音色列表
│   └── VoiceModal.vue  # 音色配置弹窗
```

---

## 八、实现计划

### Phase 1 - MVP

1. [ ] 后端 TTS 命令实现
   - [ ] `tts_speak` 命令
   - [ ] `get_tts_config` 命令
   - [ ] `save_tts_config` 命令
   - [ ] 音频缓存逻辑

2. [ ] 配置结构更新
   - [ ] 更新 `LLMConfigData` 添加 TTS 相关结构
   - [ ] 添加音色注册表结构

3. [ ] 前端 TTS 设置界面
   - [ ] TTS 主页面
   - [ ] 音色列表
   - [ ] 音色添加/编辑弹窗

4. [ ] 前端角色设置界面
   - [ ] 音色选择下拉框

5. [ ] 集成测试
   - [ ] TTS 调用和播放
   - [ ] 音色切换
   - [ ] 缓存验证

### Phase 2 - 后续迭代（暂不实现）

- [ ] 情感自动识别（根据 AI 回复内容匹配情感）
- [ ] 多语言支持
- [ ] 音频保存功能

---

## 九、待确认事项

| # | 问题 | 备注 |
|---|------|------|
| 1 | IndePTTS2 是否支持流式返回？ | 确认后可能需要修改设计 |
| 2 | 可用的 speaker 音色列表在哪里获取？ | 目前先让用户手动输入 |
| 3 | emo 参数支持哪些语义情感词？ | 目前先让用户手动输入 |

---

## 十、参考

- [IndePTTS2 项目](https://github.com/your-indeptts2-repo) （如有）
- [MiniMax TTS API 文档](https://platform.minimaxi.com/docs/api-reference/speech-t2a-intro)
