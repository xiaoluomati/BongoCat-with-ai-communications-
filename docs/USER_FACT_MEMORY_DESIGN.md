# 用户事实记忆功能调研报告 + 设计方案

> 任务ID: JJC-20260429-002
> 生成时间: 2026-04-29
> 状态: 待皇上审阅批准后方可实施

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
data/memory/facts/{user_id}_facts.json
data/memory/facts/pending_facts.jsonl   # 追加写入
data/memory/facts/archive/               # 冷数据降级
```

```rust
// 核心数据结构
pub struct UserFact {
    pub id: String,                    // UUID
    pub subject: String,                // 主体："用户"
    pub predicate: String,             // 谓语："喜欢"
    pub object: String,                // 对象："火锅"
    pub evidence: Evidence,             // 来源证据
    pub confidence: f32,               // 置信度 0.0~1.0
    pub tags: Vec<String>,             // 标签：["饮食偏好"]
    pub created_at: i64,
    pub updated_at: i64,
    pub access_count: u32,            // 访问次数（影响冷热）
}

pub struct Evidence {
    pub date: String,
    pub message_id: String,
    pub snippet: String,              // 原文片段
}

pub struct FactStore {
    pub facts: Vec<UserFact>,
    pub version: String,
}
```

**存储路径：** `data/memory/facts/{user_id}_facts.json`

**优点：**
- 零新依赖（JSON 文件）
- 与现有 memory 模块风格一致
- 千条事实内检索无压力（O(n)，1000条 < 1ms）
- 易于调试、备份

**缺点：**
- 大规模事实后需优化检索（但用户量级不会有这个问题）
- 无内置语义检索（需靠 LLM 辅助）

---

### 方案B：SQLite 存储（关系型）

**设计：**

```rust
// 使用 rusqlite 存储
CREATE TABLE facts (
    id TEXT PRIMARY KEY,
    subject TEXT NOT NULL,
    predicate TEXT NOT NULL,
    object TEXT NOT NULL,
    evidence_date TEXT,
    evidence_message_id TEXT,
    evidence_snippet TEXT,
    confidence REAL DEFAULT 1.0,
    created_at INTEGER,
    updated_at INTEGER,
    access_count INTEGER DEFAULT 0,
    is_archived INTEGER DEFAULT 0
);

CREATE INDEX idx_subject ON facts(subject);
CREATE INDEX idx_predicate ON facts(predicate);
CREATE INDEX idx_object ON facts(object);
CREATE INDEX idx_access_count ON facts(access_count);
```

**优点：**
- SQL 查询高效，支持复杂条件
- 内置事务，天然支持并发
- 成熟稳定

**缺点：**
- 引入 `rusqlite` 依赖
- 数据库迁移/调试稍复杂
- 对于千条级别，JSON 性能无差异

---

### 方案C：向量数据库（如 ChromaDB）

**设计：**

```rust
// 每条事实 → 向量嵌入
let embedding = embed_model.encode("用户喜欢火锅");
// 存储到 ChromaDB
collection.add([
    EmbeddingRecord {
        id: fact.id,
        embedding,
        document: "用户喜欢火锅",
        metadata: fact.metadata,
    }
]);

// 检索时
let results = collection.query(
    query_embeddings=[embed(user_query)],
    n_results=5
);
```

**优点：**
- 语义匹配精准（"用户说过关于吃的内容" → 能召回"火锅"）

**缺点：**
- 引入 ChromaDB + embedding 模型
- 过度工程化（几千条事实用不上）
- 部署复杂度大幅增加
- 嵌入式设备可能无法运行向量计算

---

### 方案对比总结

| 维度 | 方案A: JSON | 方案B: SQLite | 方案C: 向量DB |
|------|-------------|---------------|---------------|
| 新增依赖 | 0 | 1 (`rusqlite`) | 2+ (Chroma, embedding) |
| 存储规模 | ~10,000条 | ~1,000,000条 | ~10,000条 |
| 检索方式 | 遍历/标签 | SQL | 向量相似度 |
| 语义检索 | ❌ | ❌ | ✅ |
| 工程复杂度 | 低 | 中 | 高 |
| 部署难度 | 无 | 低 | 高 |
| 调试难度 | 低 | 中 | 高 |

**推荐：方案A（JSON）**

理由：
1. 零新增依赖
2. 用户量级决定不需要向量检索
3. 与现有 memory 模块风格一致
4. 千条事实内性能无问题

---

## 四、数据流设计

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
  Prompt: "从以下对话中提取用户事实：\n{messages}"
         ↓
  收到结构化事实列表
         ↓
  save_user_fact() → pending_facts.jsonl（追加写入）
         ↓
  merge_pending_facts()（异步，每100条或每小时）
    ├── 去重（完全匹配）
    └── 写入 facts.json
         ↓
 Facts 存储完成

检索注入流程（每次对话构建 system prompt）：
  获取相关事实
    ├── 方式1：按标签筛选（"饮食偏好"相关事实）
    └── 方式2：LLM 语义检索（给 LLM 事实列表，让它选相关）
         ↓
  注入到 system prompt
  Prompt: "以下是你已知的用户事实，请自然提起：\n{facts}"
         ↓
  对话时自然提起

冷数据降级（90天未访问）：
  facts.json → archive/{date}_facts.json
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
3. 置信度：1.0=确定，0.8=很可能，0.6=可能
4. 每条事实附带原文片段（evidence）

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

---

## 五、风险评估

| 风险 | 等级 | 缓解措施 |
|------|------|----------|
| LLM 事实提取不准确 | 🟡 中 | 置信度字段 + 用户可编辑/删除 |
| 存储膨胀 | 🟢 低 | 90天冷数据降级 + 去重机制 |
| 隐私问题 | 🟡 中 | 敏感信息检测 + 本地存储 + 用户可控清除 |
| 检索质量 | 🟡 中 | LLM 辅助语义匹配 + 标签筛选 |
| 并发写入冲突 | 🟢 低 | 追加写入 pending + Mutex 保护合并 |
| 错误事实影响对话 | 🟡 中 | 高置信度(>0.8)才注入 + 用户确认 |

---

## 六、工作量估算

### 第一阶段：核心存储（预计 8 小时）

**产出：** `facts.rs` 模块 + 基本 CRUD

| 模块 | 工时 | 内容 |
|------|------|------|
| 后端 facts.rs | 4h | CRUD + 并发保护 + 去重 |
| 前端展示（事实列表） | 3h | 查看/编辑/删除事实 |
| 集成测试 | 1h | cargo test 验证 |

### 第二阶段：自动提取（预计 6 小时）

**产出：** 对话后自动触发事实提取

| 模块 | 工时 | 内容 |
|------|------|------|
| LLM 提取调用 | 3h | Prompt + API 调用 + 解析 |
| 触发条件控制 | 2h | Throttle + 条件检查 |
| 测试 | 1h | 验证提取质量 |

### 第三阶段：智能检索（预计 4 小时）

**产出：** 对话时自然注入相关事实

| 模块 | 工时 | 内容 |
|------|------|------|
| 事实注入 system prompt | 2h | 相关事实筛选 + 格式化 |
| 冷数据降级 | 1h | 90天归档机制 |
| 测试 | 1h | 验证对话自然度 |

### 总计

| 阶段 | 工时 | 产出 |
|------|------|------|
| 第一阶段 | 8h | 核心存储 |
| 第二阶段 | 6h | 自动提取 |
| 第三阶段 | 4h | 智能检索 |
| **总计** | **18小时（~3人天）** | 完整功能 |

---

## 七、推荐方案

### 推荐：方案A（JSON 结构化存储）+ 分阶段实施

**理由：**

1. **零新依赖**：不引入任何 Rust crate
2. **与现有架构一致**：memory 模块已用 JSON 文件
3. **风险可控**：每阶段可独立验收
4. **用户量级合适**：几千条事实，JSON 检索无压力

### 分阶段目标

**Phase 1（MVP）：**
- 手动添加事实（用户可在 UI 中输入）
- 查看/编辑/删除已有事实
- 验证存储结构合理

**Phase 2（自动提取）：**
- 对话结束后自动提取事实
- 触发条件：≥3轮 + 消息长度 + 冷却时间

**Phase 3（智能注入）：**
- 对话时注入相关事实
- 桌宠能自然提起用户说过的事

---

## 八、待皇上批准事项

1. **采用方案A**（JSON 结构化存储）
2. **Phase 1 先实施**，验证后再继续
3. **事实数据存储位置**：`data/memory/facts/`
4. **触发条件**：≥3轮 + 消息≥50字符 + 冷却30分钟

---

*本报告由中书省起草，待门下省审议、皇上批准后方可实施代码修改。*
