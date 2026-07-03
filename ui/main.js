// 最小前端：调用 Tauri command 登录，监听后端 emit 的 cohub-log 事件并展示。
// 未在本环境验证（无 Tauri 运行时可测试 `window.__TAURI__` 是否按此形状暴露 API，
// 需要在真实 `cargo tauri dev` 环境里确认 invoke/listen 的实际导入路径，
// Tauri v2 标准用法应为 `window.__TAURI__.core.invoke` / `window.__TAURI__.event.listen`）。

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const logEl = document.getElementById("log");
const statusEl = document.getElementById("status");
const loginBtn = document.getElementById("login-btn");

function appendLog(message) {
  const line = document.createElement("div");
  line.className = "line";
  line.textContent = message;
  logEl.appendChild(line);
  logEl.scrollTop = logEl.scrollHeight;
}

listen("cohub-log", (event) => {
  appendLog(event.payload.message);
});

loginBtn.addEventListener("click", async () => {
  loginBtn.disabled = true;
  statusEl.textContent = "登录中...";
  try {
    await invoke("login");
    statusEl.textContent = "已连接";
  } catch (err) {
    statusEl.textContent = "登录失败";
    appendLog(`[错误] ${err}`);
  } finally {
    loginBtn.disabled = false;
  }
});

(async () => {
  const alreadyLoggedIn = await invoke("check_login");
  if (alreadyLoggedIn) {
    statusEl.textContent = "本地已有登录态（点击登录以重新连接订阅）";
  }
})();
