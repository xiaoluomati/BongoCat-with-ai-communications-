# TTS 语音合成功能设计文档

> 版本：2.0
> 日期：2026-04-01
> 状态：已实现（IndexTTS API 适配版）

---

## 一、需求概述

**目标：** 将 AI 对话内容转换为语音输出

**使用场景：** 桌宠回复时，用语音朗读 AI 的回答

**对话流程：**
```
用户发送消息 → AI 生成回复 → 回复保存到聊天 → TTS 播放语音
```

---

## 二、技术方案

### 2.1 IndexTTS API 规格

**服务端地址：** `http://localhost:9880`（默认，可配置）

**API 基础路径：** `http://localhost:9880/run/`

### 2.2 API 接口清单

| 接口 | 方法 | 路径 | 功能 |
|------|------|------|------|
| 提交合成 | POST | `/run/submit_and_refresh` | 核心 TTS 合成接口 |
| 获取音色 | POST | `/run/update_voices` | 获取可用音色列表 |
| 获取情感 | POST | `/run/update_emos` | 获取可用情感列表 |
| 清空任务 | POST | `/run/clear_all_tasks` | 清空所有任务 |

### 2.3 核心接口详解

#### 提交合成 `POST /run/submit_and_refresh`

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
  "max_tokens": 100,
  "do_sample": true,
  "temperature": 0.7,
  "top_p": 0.9,
  "top_k": 50
}
```

**参数说明：**

| 参数 | 类型 | 范围 | 必填 | 说明 |
|------|------|------|------|------|
| `voices_dropdown` | string | enum | ✅ | 音色选择 |
| `speed` | number | 0.1-2.5 | ✅ | 语速 |
| `text` | string | - | ✅ | 要合成的文本 |
| `emo_control_method` | string | enum | ❌ | 情感控制方式 |
| `emo_weight` | number | 0.0-1.6 | ❌ | 情感强度 |
| `emo_text` | string | - | ❌ | 情感描述文本 |

**emo_control_method 选项：**
1. `与音色参考音频相同`
2. `使用情感参考音频`
3. `使用情感向量控制`
4. `使用情感描述文本控制`（推荐）

**响应：** 返回包含音频文件 URL 的 JSON

---

### 2.4 可用音色

| 音色名称 | 说明 |
|----------|------|
| 使用参考音频 | 使用自定义参考音频 |
| 凯茜娅 | 内置音色 |
| 女主播 | 内置音色 |
| 无底噪 | 内置音色 |
| 苏瑶 | 内置音色 |
| 苔丝 | 内置音色 |
| 评书 | 内置音色 |

---

### 2.5 可用情感

| 情感名称 | 说明 |
|----------|------|
| 伤心.wav | 负面情感 |
| 嫌弃.wav | 负面情感 |
| 害怕.wav | 负面情感 |
| 平静.wav | 中性情感 |
| 生气.wav | 负面情感 |
| 高兴.wav | 正面情感 |

---

### 2.6 调用流程

```
1. 获取音色列表
   POST /run/update_voices
   ↓
2. 获取情感列表
   POST /run/update_emos
   ↓
3. 用户在界面选择音色和情感
   ↓
4. 提交合成任务
   POST /run/submit_and_refresh
   ↓
5. 解析响应获取音频 URL
   ↓
6. 下载/播放音频
```

---

## 三、配置设计

### 3.1 音色注册表方案

**核心设计：**
- 音色配置集中存储在 `tts.voices` 下
- 每个角色通过 `voice_id` 引用音色
- 无 `voice_id` 的角色使用全局默认音色

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
        "emo": "高兴.wav",
        "weight": 0.8,
        "emo_method": "使用情感描述文本控制",
        "speed": 1.0
      },
      "kaxiya": {
        "speaker": "凯茜娅",
        "emo": "高兴.wav",
        "weight": 0.8,
        "emo_method": "使用情感描述文本控制",
        "speed": 1.0
      }
    }
  }
}
```

### 3.3 配置项说明

**TTS 全局配置：**

| 配置项 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| `enabled` | bool | `false` | TTS 功能总开关 |
| `base_url` | string | `http://localhost:9880` | IndexTTS 服务地址 |
| `default_voice_id` | string | `suyao` | 默认音色 ID |
| `volume` | int | `80` | 音量 (0~100) |
| `speed` | float | `1.0` | 语速 (0.1~2.5) |

**音色配置 (voices)：**

| 配置项 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| `speaker` | string | - | IndexTTS 音色名称 |
| `emo` | string | `高兴.wav` | 情感标签 |
| `weight` | float | `0.8` | 情感强度 (0.0~1.6) |
| `emo_method` | string | `使用情感描述文本控制` | 情感控制方式 |
| `speed` | float | `1.0` | 语速 (0.1~2.5) |

---

## 四、缓存设计

### 4.1 缓存策略

使用 `text` + `speaker` + `emo` + `emo_method` + `speed` 的组合生成缓存 key（MD5）

### 4.2 缓存位置

系统缓存目录：
- Linux: `~/.cache/bongo-cat/tts/`
- macOS: `~/Library/Caches/bongo-cat/tts/`
- Windows: `%LOCALAPPDATA%\bongo-cat\tts\`

### 4.3 缓存格式

```
tts/
├── a1b2c3d4e5f6g7h8.wav
└── ...
```

---

## 五、后端命令

| 命令 | 功能 |
|------|------|
| `get_tts_config` | 获取 TTS 配置 |
| `save_tts_config` | 保存 TTS 配置 |
| `get_voice_config` | 获取音色配置 |
| `save_voice` | 添加/更新音色 |
| `delete_voice` | 删除音色 |
| `get_index_tts_voices` | 从服务器获取可用音色列表 |
| `get_index_tts_emos` | 从服务器获取可用情感列表 |
| `tts_speak` | 调用 TTS 朗读文本 |
| `clear_tts_cache` | 清理缓存 |
| `get_tts_cache_info` | 获取缓存信息 |

---

## 六、前端界面

### 6.1 TTS 设置页面

**入口：** 设置 → 综合功能 → 语音设置

### 6.2 UI 元素

| 元素 | 类型 | 说明 |
|------|------|------|
| 启用 TTS 开关 | Switch | 总开关 |
| 服务地址 | Input + Button | IndexTTS URL + 获取选项按钮 |
| 音量 | Slider | 0 ~ 100 |
| 连接测试 | Button | 测试连接 |
| 清理缓存 | Button | 清理缓存文件 |
| 音色列表 | List | 显示已配置的音色 |
| 添加/编辑音色 | Modal | 音色配置表单 |

### 6.3 音色配置弹窗

| 元素 | 类型 | 说明 |
|------|------|------|
| 音色 | Select/Input | 从服务器获取或手动输入 |
| 情感 | Select/Input | 从服务器获取或手动输入 |
| 情感控制方式 | Select | 4 种控制方式 |
| 情感强度 | Slider | 0.0 ~ 1.6 |
| 语速 | Slider | 0.1 ~ 2.5 |

---

## 七、实现清单

### 已完成

- [x] 后端 TTS 命令框架
- [x] 配置结构（音色注册表）
- [x] IndexTTS API 适配（POST 请求）
- [x] 获取服务器音色/情感列表
- [x] 前端 TTS 设置界面
- [x] 音色管理（添加/编辑/删除）
- [x] 音频缓存逻辑

### 待完成

- [ ] 音频实际播放（当前只有占位符）
- [ ] 角色音色绑定（角色配置中添加 voice_id 字段）
- [ ] 对话中自动触发 TTS

---

## 八、版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 1.0 | 2026-03-31 | 初始设计文档（基于错误的 GET API 假设） |
| 2.0 | 2026-04-01 | 根据 IndexTTS OpenAPI 文档更新，改为 POST 接口 |
