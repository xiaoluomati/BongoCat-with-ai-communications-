/**
 * 情感标签解析状态机
 * 解析格式：<emo>情感</emo>
 * 支持6种情感：高兴、伤心、生气、害怕、平静、嫌弃
 */

// 解析状态
export enum ParseState {
  TEXT = 'TEXT',           // 普通文本模式
  TAG_OPEN = 'TAG_OPEN',   // 刚遇到 '<'
  TAG_VERIFY = 'TAG_VERIFY', // 验证是否为 '<emo>'
  CONTENT = 'CONTENT',     // 标签内容模式
  TAG_CLOSE = 'TAG_CLOSE', // 遇到 '</'
  READY = 'READY',         // 解析完成
}

// 解析结果
export interface ParseResult {
  emotion: string      // 最后一个解析出的情感（用于TTS，向后兼容）
  emotions: string[]   // 所有解析出的情感数组
  text: string         // 累积的正文（无标签，用于显示）
  ready: boolean       // 是否解析完成
}

// 状态机上下文
export interface ParserContext {
  state: ParseState
  buffer: string       // 当前累积的文本
  tagBuffer: string    // 标签缓冲区
  emotion: string      // 已解析的情感（最后一个）
  emotions: string[]   // 所有已解析的情感数组
  text: string         // 已解析的纯文本
}

// 情感映射表
export const EMOTION_MAP: Record<string, string> = {
  '高兴': '高兴.wav',
  '开心': '高兴.wav',
  '伤心': '伤心.wav',
  '难过': '伤心.wav',
  '生气': '生气.wav',
  '愤怒': '生气.wav',
  '害怕': '害怕.wav',
  '恐惧': '害怕.wav',
  '平静': '平静.wav',
  '淡定': '平静.wav',
  '嫌弃': '嫌弃.wav',
  '厌恶': '嫌弃.wav',
}

// 默认情感
export const DEFAULT_EMOTION = '高兴.wav'

// 有效的情感关键词
const VALID_EMOTIONS = Object.keys(EMOTION_MAP)

// 创建新的解析器上下文
export function createParserContext(): ParserContext {
  return {
    state: ParseState.TEXT,
    buffer: '',
    tagBuffer: '',
    emotion: '',
    emotions: [],
    text: '',
  }
}

// 验证并规范化情感
function normalizeEmotion(emotion: string): string {
  const trimmed = emotion.trim()
  if (!trimmed) return DEFAULT_EMOTION
  
  // 直接匹配
  if (EMOTION_MAP[trimmed]) {
    return EMOTION_MAP[trimmed]
  }
  
  // 尝试查找包含关系
  for (const key of VALID_EMOTIONS) {
    if (trimmed.includes(key) || key.includes(trimmed)) {
      return EMOTION_MAP[key]
    }
  }
  
  // 默认返回高兴
  return DEFAULT_EMOTION
}

// 解析单个字符
function parseChar(ctx: ParserContext, char: string): ParserContext {
  const newCtx = { ...ctx, buffer: ctx.buffer + char }
  
  switch (ctx.state) {
    case ParseState.TEXT:
      if (char === '<') {
        newCtx.state = ParseState.TAG_OPEN
        newCtx.tagBuffer = '<'
      } else {
        newCtx.text += char
      }
      break
      
    case ParseState.TAG_OPEN:
      if (char === 'e') {
        newCtx.tagBuffer += char
        newCtx.state = ParseState.TAG_VERIFY
      } else {
        // 不是 <emo> 标签，回退到文本模式
        newCtx.text += newCtx.tagBuffer
        newCtx.tagBuffer = ''
        newCtx.state = ParseState.TEXT
        newCtx.text += char
      }
      break
      
    case ParseState.TAG_VERIFY:
      newCtx.tagBuffer += char
      if (newCtx.tagBuffer === '<emo>') {
        newCtx.state = ParseState.CONTENT
        newCtx.tagBuffer = ''
      } else if (!'<emo>'.startsWith(newCtx.tagBuffer)) {
        // 不匹配，回退
        newCtx.text += newCtx.tagBuffer
        newCtx.tagBuffer = ''
        newCtx.state = ParseState.TEXT
      }
      break
      
    case ParseState.CONTENT:
      if (char === '<') {
        // 情感内容结束，开始关闭标签
        newCtx.emotion = ctx.tagBuffer
        newCtx.tagBuffer = '<'
        newCtx.state = ParseState.TAG_CLOSE
      } else {
        newCtx.tagBuffer += char
      }
      break
      
    case ParseState.TAG_CLOSE:
      newCtx.tagBuffer += char
      if (newCtx.tagBuffer === '</emo>') {
        // 完整标签解析完成，收集情感到数组
        let normalized = normalizeEmotion(ctx.tagBuffer)
        newCtx.emotion = normalized
        newCtx.emotions = [...ctx.emotions, normalized]
        newCtx.state = ParseState.READY
        newCtx.tagBuffer = ''
      } else if (!'</emo>'.startsWith(newCtx.tagBuffer)) {
        // 关闭标签不匹配，回退到内容模式
        newCtx.tagBuffer = ctx.tagBuffer + newCtx.tagBuffer
        newCtx.state = ParseState.CONTENT
      }
      break
      
    case ParseState.READY:
      // 已经就绪，新字符进入文本模式
      newCtx.state = ParseState.TEXT
      newCtx.text += char
      break
  }
  
  return newCtx
}

// 解析文本块（支持流式）
export function parseEmotionChunk(
  ctx: ParserContext,
  chunk: string
): { context: ParserContext; result: ParseResult } {
  let currentCtx = ctx
  
  for (const char of chunk) {
    currentCtx = parseChar(currentCtx, char)
  }
  
  const result: ParseResult = {
    emotion: currentCtx.emotion || '',
    emotions: currentCtx.emotions,
    text: currentCtx.text,
    ready: currentCtx.state === ParseState.READY,
  }
  
  // 如果已就绪，重置状态以便处理下一个标签
  if (currentCtx.state === ParseState.READY) {
    currentCtx = {
      ...currentCtx,
      state: ParseState.TEXT,
      emotion: '',
      emotions: currentCtx.emotions,
      text: '',
    }
  }
  
  return { context: currentCtx, result }
}

// 一次性解析完整文本（非流式）
export function parseEmotion(text: string): ParseResult {
  const ctx = createParserContext()
  const { result } = parseEmotionChunk(ctx, text)
  return result
}

// 提取纯净文本（移除所有情感标签）
export function extractPureText(text: string): string {
  return text.replace(/<emo>.*?<\/emo>/g, '')
}

// 检查是否包含情感标签
export function hasEmotionTag(text: string): boolean {
  return /<emo>.*?<\/emo>/.test(text)
}

// 获取情感提示词（用于追加到system_prompt）
export function getEmotionPrompt(): string {
  return `你的所有回复需要在开头包含情感标签，格式如下：
<emo>情感</emo>

可选情感：高兴、伤心、生气、害怕、平静、嫌弃

注意：
1. 标签必须在回复最开头
2. 标签单独作为一个chunk输出，不要与其他内容混合
3. 不在6个中或空 → 默认"高兴"`
}

// 导出默认对象
export default {
  ParseState,
  EMOTION_MAP,
  DEFAULT_EMOTION,
  createParserContext,
  parseEmotionChunk,
  parseEmotion,
  extractPureText,
  hasEmotionTag,
  getEmotionPrompt,
}
