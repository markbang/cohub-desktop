# Cohub Desktop

Cohub 的桌面端周边应用，基于 Tauri + Rust 构建。

## 功能

- **一键登录**：通过 Device Flow 登录 Cohub 账号，无需手动配置 token。
- **实时状态通知**：登录后自动订阅你的 Space，实时展示对话的流式状态（当前第几轮模型调用），并在对话完结时提示。

## 使用

1. 打开应用，点击「登录」
2. 按提示打开链接完成授权
3. 登录成功后自动连接并开始接收所在 Space 的实时事件

## 技术栈

- Rust + Tauri
- 前端：纯 HTML/JS

## 目录结构

```
cohub-desktop/
├── Cargo.toml
├── tauri.conf.json
├── build.rs
├── src/
│   ├── main.rs      入口，注册 Tauri commands
│   ├── auth.rs       登录与 token 管理
│   ├── ws.rs          实时事件订阅
│   └── api.rs          Cohub API 调用
└── ui/
    ├── index.html
    └── main.js
```

## 开发

```bash
cargo tauri dev
```

需要 Rust 工具链与 [Tauri 前置依赖](https://tauri.app/start/prerequisites/)。

## Roadmap

- 系统级通知（当前为应用内日志展示）
- 云盘挂载：将 Space 文件挂载为本地磁盘
