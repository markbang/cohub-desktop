import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AccountInfo, AppSettings, AuthStatus, SubscriptionStatus } from "./types";

// 命令名保持 snake_case；参数名走 Tauri 默认 camelCase 转换。
export const cohub = {
  login: () => invoke<void>("login"),
  logout: () => invoke<void>("logout"),
  checkLogin: () => invoke<boolean>("check_login"),
  getAccount: () => invoke<AccountInfo | null>("get_account"),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  getSettings: () => invoke<AppSettings>("get_settings"),
  setSettings: (settings: AppSettings) =>
    invoke<void>("set_settings", { settings }),
  markFinalizedRead: () => invoke<void>("mark_finalized_read"),
};

export type EventMap = {
  "auth-status": AuthStatus;
  "subscription-status": SubscriptionStatus;
  "in-app-notify": {
    lastText?: string;
    messageCount: number;
    toolCallCount: number;
    hasError: boolean;
  };
  "settings-updated": AppSettings;
};

export function on<K extends keyof EventMap>(
  event: K,
  handler: (payload: EventMap[K]) => void,
): Promise<UnlistenFn> {
  return listen<EventMap[K]>(event, (e) => handler(e.payload));
}
