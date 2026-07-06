# AGENTS.md

本项目所有 agent / 贡献者必须遵守以下约定。

## 提交信息

提交信息（commit message）必须遵循 [Conventional Commits 1.0.0](https://www.conventionalcommits.org/en/v1.0.0/)：

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

- `type` 必须是下列之一：`feat`、`fix`、`docs`、`style`、`refactor`、`perf`、`test`、`build`、`ci`、`chore`、`revert`
- 可用 `!` 标记破坏性变更，如 `feat!: drop legacy config`
- Release notes 由 [changelogithub](https://github.com/antfu/changelogithub) 从提交信息自动生成，不规范提交会被忽略

示例：

```
feat(tray): add unread badge on macOS
fix(ws): reconnect on auth token expiry
docs: rewrite README as product overview
ci: auto-publish release on tag push
```

## UI

- 必须使用 UI 库，不得手写组件样式或裸 HTML 排版。当前选型为 [@cloudflare/kumo](https://github.com/cloudflare/kumo)（基于 Base UI + Tailwind v4），搭配 `@phosphor-icons/react`。
- 遵循项目已有主题 token（`kumo-*` 语义色），不要引入第二套配色或设计系统。
- 优先用 kumo 组件（`Button`、`LayerCard`、`Switch`、`Badge`、`Text`、`Dialog` 等）；kumo 无对应组件时再用 Base UI primitives。

## 实现原则

- 以 Rust 为主：业务逻辑、网络、WebSocket、系统托盘、文件操作一律放 `src-tauri/` 的 Rust 代码里，前端只做展示与触发。
- 前端通过 Tauri command 调用 Rust，避免在 JS 里直接发网络请求或维护状态机。
- 保持高性能：异步不阻塞 UI 线程，WS 订阅跑在独立 tokio task，通知走系统原生通道（`notify-rust`）。
- 新增依赖前评估必要性，优先复用已有 crate。
