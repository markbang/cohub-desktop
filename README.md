# Cohub Desktop

Cohub 的桌面端「周边」应用，基于 Tauri v2 + Rust + React + [Kumo](https://github.com/cloudflare/kumo)（Cloudflare 设计系统）构建。

## 能力

- **Device Flow 登录**：一键授权登录 Cohub 账号，自动打开浏览器、自动轮询，无需手动粘贴 token。启动时若有本地登录态则静默恢复。
- **实时活动通知**：登录后一次性订阅账号下**全部 Space**，实时展示每个对话的流式状态（steps / tools / model），对话完结时推送**可点击的系统通知**——点击跳转网页端对应 session，通知体显示最后一轮回复预览。
- **应用设置**：左下角账户区可进入设置，按需过滤监听范围与通知方式：
  - 仅监听我创建的 Space / 仅监听我的对话
  - 对话完结系统通知 / 托盘未读红点 / 应用内 Toast
- **系统托盘**：常驻托盘图标，有未查看的完结对话时显示未读计数；单击托盘切换窗口显示/隐藏。关闭窗口默认隐藏到托盘而非退出，软件只能从托盘「退出」结束。
- **云盘挂载（Roadmap）**：把 Space 当作云盘挂载到本地。命令链路已接通，后端 FUSE/WinFSP 实现为下一步。

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面壳 | Tauri v2（Rust） |
| 前端 | React 19 + TypeScript + Vite |
| UI 设计系统 | `@cloudflare/kumo`（Base UI + Tailwind v4） |
| 图标 | `@phosphor-icons/react` |
| 通知 | `notify-rust`（带点击 action） |
| 托盘 | Tauri `tray-icon` 特性（`TrayIconBuilder`） |
| 实时 | WebSocket（`tokio-tungstenite`），断线自动重连 |

## 目录结构

```
cohub-desktop/
├── package.json            前端依赖与脚本
├── vite.config.ts
├── tsconfig.json
├── index.html
├── pnpm-workspace.yaml     pnpm 构建白名单（esbuild）
├── src/                    前端源码
│   ├── main.tsx
│   ├── App.tsx
│   ├── styles.css          kumo 主题 + tailwind v4 入口
│   ├── state.ts            useCohub：事件订阅 + reducer
│   ├── lib/
│   │   ├── api.ts          invoke 封装 + 事件监听
│   │   ├── types.ts        与 Rust 事件 payload 对齐的 TS 类型
│   │   └── toast.ts        模块级 toast manager
│   └── components/
│       ├── Sidebar.tsx
│       ├── SignInCard.tsx
│       ├── LiveActivity.tsx
│       ├── TurnCard.tsx
│       ├── ActivityFeed.tsx
│       └── CloudDrive.tsx
└── src-tauri/
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── capabilities/main.json   权限：core:default
    ├── build.rs
    └── src/
        ├── main.rs         入口，commands + 事件转发 + 通知 + 自动恢复
        ├── auth.rs         device flow / token 刷新 / 登出
        ├── settings.rs     应用设置持久化（~/.config/cohub-desktop/settings.json）
        ├── tray.rs         系统托盘 + 未读红点
        ├── api.rs          /api/me + /api/spaces（含 owner）
        └── ws.rs           WS 订阅全部 Space，解析 turn/activity 事件， session→space 映射回退
```

## 开发

```bash
pnpm install
cargo tauri dev
```

需要 Rust 工具链、Tauri 前置依赖（Linux 需 `webkit2gtk-4.1`、`gtk4`）。

## 发布

打 tag 触发 GitHub Actions 自动构建并发布跨平台安装包（draft release）：

```bash
git tag v0.0.1
git push origin v0.0.1
```

CI（`.github/workflows/release.yml`）会在 macOS（Apple Silicon + Intel）、Windows、Linux 上并行构建，产物上传到 GitHub Release。发布前在 Release 页编辑 notes 后点 Publish。

## 图标

应用图标由 cohub 官方 favicon 生成。更新图标：

```bash
rsvg-convert -w 1024 ../cohub/apps/web/static/favicon.svg -o /tmp/icon.png
cargo tauri icon /tmp/icon.png -o src-tauri/icons
```

## 认证流程

Device Flow（RFC 8628），端点已实测：

1. `POST https://auth.neta.art/oidc/device/auth` → 拿 device_code + user_code
2. 自动打开 `verification_uri_complete`，前端展示用户码供复制
3. 轮询 `POST https://auth.neta.art/oidc/token`（`grant_type=urn:ietf:params:oauth:grant-type:device_code`）
4. 成功后存 `~/.config/cohub-desktop/session.json`，拿账号 + 全部 Space，开 WS 订阅

## 事件协议

后端 emit、前端 `listen`：

| 事件 | payload | 说明 |
|---|---|---|
| `auth-status` | `{ phase, user_code?, verification_uri?, account?, error? }` | 登录全流程状态机 |
| `spaces` | `{ spaces: [{id, name}] }` | Space 列表 |
| `subscription-status` | `{ phase, space_count, error? }` | WS 连接状态 |
| `turn-progress` | `{ turn_id, session_id, steps, tools, llm_round, model, … }` | 流式进度（累积 patch 的 messageOrdinal / tool_use） |
| `turn-finalized` | `{ turn_id, session_id, sequence, message_count, tool_call_count, usage_input, usage_cache_read, usage_output, duration_ms, last_text, … }` | 完结快照（来自 turn.intermediateSummary） |
| `activity` | `{ eventType, spaceId?, sessionId? }` | 原始事件流（最多保留 120 条） |

### 完结通知

`session.turn.finalized` 触发一条系统通知（`notify-rust`，带 `default` action）：
- **点击通知体** → 打开 `https://cohub.run/spaces/{spaceId}/sessions/{sessionId}?turn={sequence}`
- **通知体** = `intermediateSummary.lastMessageText`（回退 `assistantText`），超 100 字截断加 “…”

进行中对话展示对齐 web `ProcessCard`：`4 steps · 4 tools · ↑3.4M (3.4M cached) ↓1.3k · 46s`。

## Roadmap

- [ ] 云盘挂载：FUSE（macOS/Linux）+ WinFSP（Windows）把 Space 挂载为本地磁盘
- [ ] 命令名 / 状态持久化：记住上次选中的视图
