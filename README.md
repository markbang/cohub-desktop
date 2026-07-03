# Cohub Desktop（Tauri）—— 最小闭环原型

> **未在本环境编译验证**：当前沙箱无 Rust/Cargo 工具链，本项目只写出骨架代码，未运行过 `cargo build` / `cargo tauri dev`。在你本地有 Rust + Tauri 工具链的机器上验证。

## 这是什么

Cohub 桌面端"周边"应用的最小闭环原型：

1. 点击「登录」→ 走 Logto Device Flow（弹出/打印登录链接）→ 轮询直到授权完成 → token 存到本地文件
2. 登录后自动连接 WebSocket，订阅当前账号可见的某个 Space
3. 收到 `session.turn.lifecycle`（含 `llmRound`）和 `session.turn.finalized` 事件时，在窗口日志区域打印一行

**不包含**：真正的系统通知（`tauri-plugin-notification` 等），因为无法在本环境验证跨平台通知代码是否正确编译/工作。日志先打印到窗口内的滚动列表，验证通过后再加系统通知是很小的改动。

**不包含**：云盘挂载（FUSE/WinFsp），按之前约定这是桌面端第二阶段，且需要额外的 native 依赖，风险更高，留到协议验证过的登录+通知闭环先跑通之后再做。

## 协议来源

所有端点、字段名、事件格式均来自 `docs/experiments/2026-07-02-cohub-companion-apps.md` 中已经用 Node.js 脚本实测验证过的结果，不是猜测或凭空实现。

## 目录结构

```
cohub-desktop/
├── README.md              本文件
├── Cargo.toml              Rust 依赖声明（workspace 根）
├── tauri.conf.json         Tauri 应用配置
├── build.rs                 Tauri 构建脚本
├── src/                     Rust 后端
│   ├── main.rs               入口，注册 Tauri commands
│   ├── auth.rs                Device flow 登录 + token 存储/刷新
│   ├── ws.rs                   WebSocket 客户端：认证、订阅、事件解析
│   └── api.rs                   最小 HTTP client（/api/me、/api/spaces）
└── ui/                       前端（纯 HTML/JS，无框架）
    ├── index.html
    └── main.js
```

## 如何在本地验证（你需要做的）

前置条件：安装 Rust、`cargo`、[Tauri 前置依赖](https://tauri.app/start/prerequisites/)（不同平台不同，比如 macOS 需要 Xcode Command Line Tools，Linux 需要 `webkit2gtk` 等）、以及 `cargo install tauri-cli` 或用 `cargo tauri`。

```bash
cd experiments/cohub-desktop
cargo tauri dev
```

预期看到一个窗口，点击「登录」按钮后：

1. 终端 / 窗口日志区出现登录链接和用户码
2. 打开链接完成授权后，窗口提示"已登录"
3. 自动连接 WebSocket，窗口日志区开始滚动显示 `[space room 已订阅]` 等状态
4. 在任意已登录账号可见的 Space 里触发一次 Cohub 对话（比如用 `cohub spaces prompt`），几秒内应该能在窗口日志区看到类似：
   ```
   [turn] llmRound=1 provider=cohub model=claude-sonnet-5 phase=llm_call_started
   [turn] ✅ finalized status=completed
   ```

## 已知限制 / 未验证项

- 未验证能否编译通过（Rust 依赖版本、Tauri API 用法可能需要微调）。
- 未验证 WebSocket 断线重连逻辑的实际行为（代码里有基础重连，但未跑过真实断网场景）。
- token 明文存储在本地文件（`~/.config/cohub-desktop/session.json`），未加密。生产化前需要加固（如系统 keychain）。
- 只订阅一个 Space（硬编码从 `/api/spaces` 拿到的第一个），未做多 Space 管理 UI。
- 未做真正系统通知，仅窗口内日志。
