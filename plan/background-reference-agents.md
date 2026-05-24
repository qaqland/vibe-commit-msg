# 背景：参考的成熟 Agent 项目及关键模式

来源：GitHub issue #3 和 #4

## 项目清单

### 1. OpenCode (v1.14.33)
- **仓库**：https://github.com/anomalyco/opencode
- **类型**：通用编码 agent
- **关键模式**：
  - AGENTS.md 初始化流程：先读高价值源（README、manifest、CI config），按信号密度排序
  - review 模式："Diffs alone are not enough" —— 必须读完整文件上下文才能判断变更正确性
  - "Be certain" 原则：不确定就调查，而非标记为 bug
  - Don't be a zealot about style：验证实际违反，不抱怨可接受的变通
- **提示词文件**：
  - `packages/opencode/src/command/template/initialize.txt`
  - `packages/opencode/src/command/template/review.txt`

### 2. Claude Code (v2.1.126)
- **仓库**：https://github.com/Piebald-AI/claude-code-system-prompts
- **类型**：通用编码 agent（系统提示词提取）
- **关键模式**：
  - Explore subagent：专用只读探索 agent，支持 quick/medium/thorough 三级深度
  - 明确工具使用边界（只读工具 vs 可写工具）
  - 多个 explore agent 可并行启动提升效率
  - subagent preference：有已有 context 的 subagent 优先 resume 而非创建新实例
- **提示词文件**：
  - `system-prompts/agent-prompt-claudemd-creation.md`
  - `system-prompts/agent-prompt-explore.md`

### 3. Kimi CLI (v1.41.0)
- **仓库**：https://github.com/MoonshotAI/kimi-cli
- **类型**：通用编码 agent
- **关键模式**：
  - AGENTS.md 生成：结构化探索后输出
  - Agent 工具描述：显式区分研究 vs 编码场景
  - subagent prefer resume over new instance
- **提示词文件**：
  - `src/kimi_cli/prompts/init.md`
  - `src/kimi_cli/tools/agent/description.md`

### 4. repo-analyzer
- **仓库**：https://github.com/yzddmr6/repo-analyzer
- **类型**：项目架构分析（Claude Code skill 插件）
- **关键模式**：
  - 8 阶段流水线：获取→规模评估→文档研究→特征识别→结构设计→并行深度分析→交叉验证→融合输出
  - 核心原则："Why > What"、全局关联、灵感式写作
  - 深度模式：Quick / Standard / Deep，按代码行数和覆盖度目标选择
  - 子 agent 并行分析 + 交叉验证 + 主 agent 质控
  - 子 agent 带结构化模板输出（10 段核心模块模板、5 段次要模块批量模板）
- **提示词文件**：
  - `skills/repo-analyzer/SKILL.md`
  - `skills/repo-analyzer/references/analysis-guide.md`
  - `skills/repo-analyzer/references/module-analysis-guide.md`

### 5. sashiko
- **仓库**：https://github.com/sashiko-dev/sashiko
- **类型**：Linux kernel 代码 review 系统
- **关键模式**：
  - 10 阶段专业化 review：目标分析→实现验证→执行流→资源管理→锁定→安全→硬件→去重→验证→报告
  - 动态阶段规划：Stage 1-3 必跑，Stage 4-7 按需跳过（"err on the side of running more stages"）
  - Stage 9 验证 = 证伪：agent 必须找到 concrete proof 才能推翻发现，"debate yourself" 步骤
  - false-positive-guide.md：15 类误报预防清单 + 10 步正向验证
  - Phase 0 预筛选：根据 patch 内容动态选择子系统专家
- **提示词文件**：
  - `src/worker/prompts.rs`（10 阶段 system prompt 内联）
  - `third_party/prompts/kernel/*.md`（54 个子系统专家 + 通用模式）

### 6. nitpicker
- **仓库**：https://github.com/arsenyinfo/nitpicker
- **类型**：代码 review 工具
- **关键模式**：
  - Parallel review：多 reviewer 并行 + aggregator 去重合并
  - Debate 模式：Actor/Critic 对抗——Actor 找问题，Critic 证伪，Meta 合成
  - `FINAL_TURN_WRAP_UP_PROMPT`：预算耗尽时强制收尾
  - Compaction（上下文压缩）：结构化摘要保留 Review Goal / Key Findings / Open Questions
  - 子 agent 委派指引含正确/错误用法示例
- **提示词文件**：
  - `src/prompts.rs`（主 prompt 文件）
  - `src/agent.rs`（wrap-up prompt + spawn_subagent 工具定义）
  - `src/compact.rs`（上下文压缩指令）

### 7. inspect
- **仓库**：https://github.com/Ataraxy-Labs/inspect
- **类型**：代码 review 工具
- **关键模式**：
  - Diff 截断策略：按编辑密度评分，deprioritize test(×0.3), doc(×0.2), snap(×0.1), config(×0.5)，65K char budget填充
  - 实体分流：risk_score × blast_radius × public_api × entity_type × change_type 加权，Top 10 给 800 char BEFORE/AFTER，Next 15 给 name-only
  - 两阶段流水线：9 并行 lens（含不同温度）→ merge + dedup → 结构性文件过滤 → 盲验证
  - Phase 2 代理挑战：agent 使用工具尝试证伪每个发现，只保留无法证伪的
  - 置信度阈值 >90%
- **提示词文件**：
  - `crates/inspect-core/src/llm.rs`
  - `crates/inspect-api/src/prompts.rs`

### 8. GitNexus
- **仓库**：https://github.com/abhigyanpatwari/GitNexus
- **类型**：代码知识图谱 + review
- **关键模式**：
  - detect_changes → context → impact → summarize 工作流
  - 影响范围按深度分层：d=1 WILL BREAK, d=2 LIKELY AFFECTED, d=3 MAY NEED TESTING
  - 风险分级：LOW / MEDIUM / HIGH / CRITICAL
  - Wiki 生成 4 阶段：分组 → 叶节点文档 → 父节点合成 → 顶层概览
- **提示词文件**：
  - `gitnexus/src/mcp/tools.ts`
  - `gitnexus/src/mcp/server.ts`
  - `gitnexus/Core/wiki/generator.ts`

### 9. lumen
- **仓库**：https://github.com/jnsahaj/lumen
- **类型**：Git 辅助 CLI
- **关键模式**：
  - draft 模式：整个响应直接传入 git commit（零冗余约束）
  - Conventional Commits 类型注入 JSON 配置
  - 极简 prompt 风格
- **提示词文件**：
  - `src/ai_prompt.rs`

### 10. repomix
- **仓库**：https://github.com/yamadashy/repomix
- **类型**：代码打包工具（为 LLM 消费优化）
- **关键模式**：
  - 输出结构：summary → tree → files
  - Skill 系统：拆分为 summary/project-structure/files 三个引用文件
  - 技术栈自动检测（从 package.json/Cargo.toml/go.mod 推断）
  - XML/Markdown/Plain 三种输出格式
- **提示词文件**：
  - `src/core/output/outputStyles/*.ts`
  - `src/core/skill/skillStyle.ts`

## 跨项目通用模式抽象

1. **"Diffs alone are not enough"**（OpenCode, sashiko）：diff 歧义时必须读完整文件上下文
2. **"Why > What"**（repo-analyzer）：每个描述必须解释变更原因而非仅仅描述变更内容
3. **深度自适应**（repo-analyzer, Claude Code）：根据变更规模适配分析深度
4. **噪声过滤**（inspect, sashiko）：lock/generated/vendored/test-snapshot 按权重降级或排除
5. **验证 = 证伪**（sashiko, nitpicker, inspect）：自检时寻找反证而非确认已有发现
6. **预算感知收尾**（nitpicker）：token 预算紧张时基于现有证据收尾，不做更多探索
7. **Subagent 分层策略**（Claude Code, nitpicker, Kimi）：3+ 步探索用 subagent，单步查找用本地工具；已有 context 优先 resume
8. **阶段动态规划**（sashiko）：固定必修阶段 + 可选专业阶段，宁可多跑不多漏
9. **并行 + 去重 + 验证三段式**（sashiko, inspect, nitpicker）：并行产出 → 去重合并 → 证伪验证
10. **结构化输出强制**（所有项目）：JSON schema 或格式化模板，模型只填内容
