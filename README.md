# Cohub Desktop

Cohub 的桌面伴侣应用，常驻系统托盘，实时感知你在 [cohub.run](https://cohub.run) 的所有对话动态。

## 它能做什么

- **实时活动监听**：后台连接你的全部 Space，追踪每个对话的流式状态——当前第几轮、调用了哪些工具、用的是什么模型。
- **完结提醒**：对话结束时推送一条系统通知，显示最后一轮回复预览，**点击通知直接跳转到网页端对应对话**。
- **托盘未读红点**：有未查看的完结对话时，托盘图标显示未读计数；打开窗口即清零。
- **轻量后台运行**：关闭窗口自动隐藏到托盘，监听不中断；软件只能从托盘「退出」结束。

## 登录

首次打开会显示登录窗口，点击「开始登录」后在浏览器完成 Cohub 授权即可，无需手动配置 token。之后启动会静默恢复登录态，直接驻留托盘。

## 偏好设置

从托盘打开窗口可调整监听范围与通知方式：

- 仅监听我创建的 Space / 仅监听我的对话
- 对话完结系统通知 / 托盘未读红点 / 应用内 Toast

## 下载

前往 [Releases](https://github.com/markbang/cohub-desktop/releases) 下载最新版本：

- **macOS** — `.dmg`（Apple Silicon 选 `aarch64`，Intel 选 `x64`）
- **Windows** — `.msi` 或 `.exe`
- **Linux** — `.deb` / `.rpm` / `.AppImage`

> 关于 Cohub 本身，请访问 [cohub.run](https://cohub.run)。
