# 用户事实记忆功能调研报告 + 设计方案

> 任务ID: JJC-20260429-002
> 生成时间: 2026-04-29
> 更新时间: 2026-04-30
> 状态: 已批准实施（含6项改进建议）

---

## 一、调研现状

### 1.1 当前 Memory 模块数据流

```
用户消息
  ↓
save_chat_message()
  ↓
data/memory/chat/{YYYY-MM-DD}.json（每日一个文件）
  ↓
DayChat { date, messages: [{id, role, content, timestamp}] }
  ↓
无事实提取，无语义检索
```

**现有存储结构：**

| 数据类型 | 路径 | 触发时机 | 内容 |
|----------|------|----------|------|
| 原始聊天 | `memory/chat/{date}.json` | 每条消息 | 纯文本，无结构化 |
| 周摘要 | `memory/weekly/{week}.json` | 每周 | 关键词+情感弧+总结 |
| 月摘要 | `memory/monthly/{month}.json` | 每月 | 话题+关系+里程碑 |
| 用户画像 | `profile/user_profile.json` | LLM分析触发 | 特征+偏好+重要日期 |

### 1.2 当前 UserProfile 模块

**文件：** `src-tauri/src/commands/character.rs`

```rust
pub struct UserProfile {
    pub user_name: Option<String>,
    pub traits: Vec<String>,              // 用户特征
    pub preferences: HashMap<String, String>,  // 偏好
    pub important_dates: HashMap,          // 重要日期
    pub recent_interactions: Vec<Interaction>,  // 最近互动
    pub special_memories: Vec<SpecialMemory>,   // 专属回忆
    pub conversation_count: u32,
    pub last_updated: String,
}
```

**UserProfile 与 Memory 的关系：**

```
build_full_context() 构建 system prompt 时：
  1. role_preset — 角色设定
  2. user_profile — 用户画像（从 user_profile.json 读取）
  3. long_term_memory — 长期记忆（周/月摘要）
  4. short_term_memory — 最近N条消息
  5. current dialog — 当前对话
```

### 1.3 现有机制为何做不到"记住用户说过的事实"

| 机制 | 能做到 | 不能做到 |
|------|--------|----------|
| 原始消息存储 | 保存每一句话 | 提取实体、关系 |
| 周/月摘要 | 话题/情感趋势 | 具体事实（"用户喜欢火锅"） |
| UserProfile | 手动填写特征 | 自动从对话学习 |
| SpecialMemory | 记录重要事件 | 从对话自动提取 |

**核心问题：**

1. **无事实提取**：消息原样存储，LLM 每次都要从原始对话中自己理解用户背景
2. **无结构化检索**：只能按日期查，无法语义搜索"用户提到过什么"
3. **UserProfile 需手动维护**：无法从对话自动学习
4. **上下文长度限制**：长期记忆注入受限于 LLM context 长度

---

## 二、问题分析

### 2.1 用户期望 vs 现有能力

**用户期望：**
> "我上周说想吃火锅，桌宠下次能主动问我火锅好吃吗"

**现有机制能做到：**
- 用户画像可以手动填"用户喜欢吃火锅"（但用户不会主动填）
- 周摘要可以总结"本周讨论了火锅"（但不是事实，是话题）
- 没有任何机制记住"用户说过想吃火锅"这个具体事实

### 2.2 技术障碍

1. **事实 vs 摘要 vs 观点 区别**
   - 事实：`"用户说想吃火锅"` — 可提取为结构化 `subject=用户, predicate=想吃, object=火锅`
   - 摘要：`"本周用户关注饮食话题"` — 是聚合结果，不是原子事实
   - 观点：`"用户似乎喜欢辣的食物"` — 置信度低，需多次确认

2. **隐私问题**
   - 用户可能在对话中透露地址/电话/财务等敏感信息
   - 存储需区分：可用于注入 context vs 仅存档

3. **存储膨胀**
   - 每天100条对话，若每5条产生1个事实，每天20个事实
   - 1年后 7300 个事实，如何高效检索？

---

## 三、技术方案

### 方案A：JSON 结构化存储（轻量）✅ 推荐

**存储结构：**

```
data/memory/facts/
  pending_facts.jsonl      # 追加写入，待确认队列
  facts.json               # 已确认事实（active 状态）
  archive/                  # 冷数据降级
```

```rust
// 核心数据结构
pub struct UserFact {
    pub id: String,                    // UUID
    pub subject: String,                // 主体："用户"
    pub predicate: String,             // 谓语："喜欢"
    pub object: String,                // 对象："火锅"
    pub evidence: Vec<Evidence>,      // 来源证据列表（可能多次确认）
    pub confidence: f32,               // 置信度 0.0~1.0
    pub status: FactStatus,           // active | superseded | deleted
    pub superseded_by: Option<String>, // 被哪个事实替代
    pub replaces: Option<String>,      // 替代了哪个旧事实
    pub tags: Vec<String>,             // 标签：["饮食偏好"]
    pub created_at: i64,
    pub updated_at: i64,
    pub access_count: i32,            // 访问次数（影响冷热）
}

pub enum FactStatus {
    Active,
    Superseded,
    Deleted,
}

pub struct Evidence {
    pub date: String,
    pub message_id: String,
    pub snippet: String,              // 原文片段
}
```

**存储路径：** `data/memory/facts/facts.json`（单用户，固定文件名）

**优点：**
- 零新依赖（JSON 文件）
- 与现有 memory 模块风格一致
- 千条事实内检索无压力（O(n)，1000条 < 1ms）
- 易于调试、备份

**缺点：**
- 大规模事实后需优化检索（但用户量级不会有这个问题）
- 无内置语义检索（需靠 LLM 辅助）

---

## 四、数据流设计（完整版）

### 4.1 完整数据流

```
┌─────────────────────────────────────────────────────────────────────┐
│                         用户事实记忆完整流程                           │
└─────────────────────────────────────────────────────────────────────┘

对话流程：
  用户消息 → chat.ts → LLM响应 → save_chat_message()

事实提取流程（对话结束后）：
  检测触发条件
    ├── 条件1：对话轮次 ≥ 3
    ├── 条件2：用户消息 ≥ 20字符
    └── 条件3：距离上次提取 ≥ 30分钟
         ↓
  积累最近5轮用户消息
         ↓
  调用 LLM 提取事实（batch）
         ↓
  敏感信息检测 → 匹配屏蔽列表 → 丢弃
         ↓
  收到结构化事实列表
         ↓
  save_user_fact() → pending_facts.jsonl（追加写入）
         ↓
  merge_pending_facts()（定时任务，每20条或每10分钟）
    ├── 置信度 < 0.7 → 标记为待确认，不合并到 facts.json
    ├── 完全匹配 → 更新 access_count
    ├── subject+predicate 相同但 object 不同 → 冲突处理
    │     ├── 旧事实改为 superseded
    │     ├── 新事实 active
    │     └── 建立 superseded_by / replaces 关联
    └── 合并写入 facts.json
         ↓
 Facts 存储完成

UserProfile 同步（合并阶段）：
  predicate ∈ {"喜欢", "不喜欢", "职业", "生日", "地点", "工作"} → 同步更新 UserProfile
         ↓
  build_full_context() 时同时注入 user_profile + active facts

检索注入流程（每次对话构建 system prompt）：
  获取相关事实
    ├── 方式1：最近7天 + 高访问次数（简单策略）
    └── 方式2：候选20条 → 轻量LLM筛选到3-5条
         ↓
  注入到 system prompt
         ↓
  对话时自然提起
```

### 4.2 触发条件设计

```rust
// 三个条件同时满足才触发提取
fn should_extract_facts(messages: &[ChatMessage], last_extract_ts: u64) -> bool {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    
    // 条件1：至少3轮对话
    let round_count = messages.iter().filter(|m| m.role == "user").count();
    if round_count < 3 { return false; }
    
    // 条件2：用户消息总长度 ≥ 50字符
    let total_chars: usize = messages.iter()
        .filter(|m| m.role == "user")
        .map(|m| m.content.len())
        .sum();
    if total_chars < 50 { return false; }
    
    // 条件3：距离上次提取 ≥ 30分钟
    if now < last_extract_ts + 1800 { return false; }
    
    true
}
```

### 4.3 LLM 事实提取 Prompt

```
你是一个信息提取助手。请从对话历史中提取关于用户的事实。

规则：
1. 只提取关于"用户"的事实（不是关于桌宠或其他人）
2. 事实格式：subject + predicate + object
3. 置信度：1.0=确定，0.8=很可能，0.6=可能，<0.6=不确定
4. 每条事实附带原文片段（evidence）
5. 排除明显敏感信息：地址、手机号、密码、银行卡号等

输出JSON格式：
{
  "facts": [
    {
      "subject": "用户",
      "predicate": "喜欢",
      "object": "火锅",
      "confidence": 0.9,
      "tags": ["饮食偏好"],
      "evidence": {
        "date": "2026-04-29",
        "message_id": "msg_123",
        "snippet": "我最近特别想吃火锅"
      }
    }
  ]
}

对话历史：
{messages}
```

### 4.4 敏感信息检测

```rust
// 敏感谓词屏蔽列表
const SENSITIVE_PREDICATES: &[&str] = &[
    "住在", "地址", "密码", "银行卡", "账号", "手机号", "电话",
];

// 敏感信息正则
const SENSITIVE_PATTERNS: &[Regex] = &[
    Regex::new(r"\d{11}").unwrap(),  // 手机号
    Regex::new(r"\d{16,}").unwrap(), // 银行卡号
];

fn is_sensitive_fact(predicate: &str, object: &str) -> bool {
    // 检查谓词
    for sp in SENSITIVE_PREDICATES {
        if predicate.contains(sp) {
            return true;
        }
    }
    // 检查对象内容
    for pattern in SENSITIVE_PATTERNS {
        if pattern.is_match(object) {
            return true;
        }
    }
    false
}
```

### 4.5 冲突处理（superseded 模式）

```
场景：用户说"我喜欢火锅"，后来又说"我讨厌火锅"

处理流程：
1. 新事实 B (predicate=讨厌, object=火锅) 进入 merge
2. 发现已有事实 A (predicate=喜欢, object=火锅)
3. 两者 subject+predicate 相同，object 不同 → 冲突
4. 操作：
   - A.status = Superseded
   - A.superseded_by = B.id
   - B.status = Active
   - B.replaces = A.id
5. 检索时只返回 B（A 被隐藏但保留可追溯）

场景：用户重复说"我喜欢火锅"（完全相同）
→ 只更新 A 的 updated_at 和 access_count，不创建新事实
```

### 4.6 检索策略（混合模式）

```rust
fn get_relevant_facts(context: &str, limit: usize) -> Vec<UserFact> {
    // 策略1：时间衰减 + 访问频率
    let candidates: Vec<UserFact> = facts
        .into_iter()
        .filter(|f| f.status == FactStatus::Active)
        .filter(|f| f.created_at > now() - 7 * 24 * 3600)  // 7天内
        .sorted_by_key(|f| -(f.access_count as i64))       // 访问次数降序
        .take(20)                                           // 候选20条
        .collect();
    
    // 策略2：关键词匹配（轻量，无需LLM）
    let keywords = extract_keywords(context);  // 从当前对话提取
    let keyword_matched: Vec<UserFact> = candidates
        .iter()
        .filter(|f| {
            f.object.contains_any(keywords) || 
            f.predicate.contains_any(keywords) ||
            f.tags.contains_any(keywords)
        })
        .take(5)
        .cloned()
        .collect();
    
    if keyword_matched.len() >= 3 {
        return keyword_matched;  // 够用就返回
    }
    
    // 策略3：轻量LLM筛选（仅在候选不足时）
    // 调用 gpt-3.5-turbo 从20条候选中选最相关3-5条
    return lightweight_llm_filter(candidates, context, limit);
}
```

---

## 五、风险评估

| 风险 | 等级 | 缓解措施 |
|------|------|----------|
| LLM 事实提取不准确 | 🟡 中 | 置信度字段 + 低置信度 pending 确认机制 |
| 存储膨胀 | 🟢 低 | 90天冷数据降级 + 去重机制 + superseded 状态 |
| 隐私问题 | 🟡 中 | 敏感谓词屏蔽 + 正则检测 + 本地存储 + 用户可控清除 |
| 检索质量 | 🟡 中 | 混合检索策略（时间衰减+关键词+LLM辅助） |
| 并发写入冲突 | 🟢 低 | tokio::sync::Mutex 保护 + 追加写入 pending |
| 错误事实影响对话 | 🟡 中 | 高置信度(>0.7)才注入 + pending 确认机制 |
| 事实冲突 | 🟡 中 | superseded 模式保留历史可追溯 |

---

## 六、工作量估算

### Phase 1.1：核心基础设施（预计 4 小时）

**产出：** facts.rs 完善 + 并发安全 + 敏感检测

| 模块 | 工时 | 内容 |
|------|------|------|
| tokio::sync::Mutex | 1h | 替换 std::sync::Mutex + 定时合并 |
| 敏感信息检测 | 1h | 谓词屏蔽 + 正则过滤 |
| 定时合并任务 | 2h | 每20条或每10分钟触发一次 |

### Phase 1.2：数据质量保障（预计 6 小时）

**产出：** pending 队列 + 冲突处理

| 模块 | 工时 | 内容 |
|------|------|------|
| 待确认事实队列 | 3h | confidence < 0.7 进入 pending，卡片展示 |
| 冲突处理机制 | 3h | status字段 + superseded_by + replaces |

### Phase 2：智能检索（预计 6 小时）

**产出：** 检索注入 + UserProfile 同步

| 模块 | 工时 | 内容 |
|------|------|------|
| 混合检索策略 | 3h | 时间衰减+关键词+LLM筛选 |
| UserProfile同步 | 3h | 合并时同步 predicate 属于预设类型 |

### 总计

| 阶段 | 工时 | 产出 |
|------|------|------|
| Phase 1.1 | 4h | 核心基础设施 |
| Phase 1.2 | 6h | 数据质量保障 |
| Phase 2 | 6h | 智能检索 |
| **总计** | **16小时（~3人天）** | 完整功能 |

---

## 七、推荐方案

### 推荐：方案A（JSON 结构化存储）+ 分阶段实施

**理由：**

1. **零新依赖**：不引入任何 Rust crate
2. **与现有架构一致**：memory 模块已用 JSON 文件
3. **风险可控**：每阶段可独立验收
4. **用户量级合适**：几千条事实，JSON 检索无压力

### 分阶段目标

**Phase 1.1（MVP）：**
- tokio::sync::Mutex 替代 std::sync::Mutex
- 定时合并（每20条或每10分钟）
- 敏感信息检测过滤

**Phase 1.2：**
- pending 待确认队列（confidence < 0.7）
- 冲突处理（superseded 模式）
- 前端展示待确认卡片

**Phase 2：**
- 混合检索策略
- UserProfile 同步
- 冷数据降级

---

## 八、已批准实施内容

| 编号 | 建议内容 | 状态 |
|------|----------|------|
| 建议1 | 低置信度事实分流（pending队列）+ 敏感谓词屏蔽 | ✅ 已采纳 |
| 建议2 | 事实冲突处理（superseded模式） | ✅ 已采纳 |
| 建议3 | 检索策略细化（混合模式） | ✅ 已采纳 |
| 建议4 | tokio::sync::Mutex + 定时合并 | ✅ 已采纳 |
| 建议5 | UserProfile 同步 | ✅ 已采纳 |
| 建议6 | 敏感信息检测 | ✅ 已采纳 |

---

*本报告由中书省起草，已整合6项改进建议。*