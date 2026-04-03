# TTS 情感识别自动匹配 - 执行方案

## 任务ID
JJC-20260402-001

## 方案概述
实现 LLM 回复内嵌情感标签 `<emo>xxx</emo>` 的解析与 TTS 情感匹配功能。

## 实现步骤

### 1. 情感解析状态机 (src/utils/emotion.ts)
- 新建文件
- 实现状态机解析 `<emo>xxx</emo>` 标签
- 返回 `{ emotion: string, text: string, ready: boolean }`
- 支持6种情感：高兴、伤心、生气、害怕、平静、嫌弃

### 2. 流式处理集成 (src/stores/chat.ts)
- 修改流式消息处理逻辑
- 集成情感解析状态机
- 消息 content 只追加纯文本（无标签）
- 触发 TTS 时传入解析出的情感

### 3. TTS 改造 (src/stores/tts.ts)
- 新增 `speakWithEmotion(text, emotion)` 方法
- 根据情感选择对应音频文件

### 4. 配置扩展 (src-tauri/src/commands/config.rs)
- TTSConfig 添加 `emotion_auto: bool`
- 控制是否启用自动情感识别

### 5. 提示词自动追加
- 开启 TTS 时，自动在 system_prompt 末尾追加情感标签要求
- 提示词内容见设计文档

## 涉及文件
1. src/utils/emotion.ts (新建)
2. src/stores/chat.ts (修改)
3. src/stores/tts.ts (修改)
4. src-tauri/src/commands/config.rs (修改)

## 预期产出
- 完整的情感解析功能
- 流式/非流式处理均支持
- 配置开关控制
- 聊天记录不保留情感标签
