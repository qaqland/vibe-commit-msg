# vibe-commit-msg

又是一个 LLM 相关的 Commit Helper

## 生命周期

计划在 prepare-commit-msg 之后，启动编辑之前运行，为用户准备 vibe 的参考。

老版本 Git 上可以通过修改 `GIT_EDITOR` 或 `core.editor` 到本项目，
等待用户提交时自动调用，完成任务后写入 `.git/COMMIT_EDITMSG` 并调用原 EDITOR
以便用户再对生成内容作修改。

新版本 Git 配置 prepare-commit-msg 更方便（因为新版本支持了多 Hook）

PS：MVP 直接输出到 STDOUT，暂未实现其它行为

## 多智能体

没有使用 PLAN/TODO/SUBAGENT 而是手动控制的流水线：

- Summary Agent 总结并缓存项目整体架构与模块划分
- Style Agent 理解 Commit 约定与文法风格
- Commit Agent 分析 `git-diff` 并结合项目信息进行理解
- Message Agent 综上所述给出提交信息的参考输出

## TODO

- Tests for tools
