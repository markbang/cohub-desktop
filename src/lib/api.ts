import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AccountInfo,
  ActivityItem,
  AppSettings,
  AuthStatus,
  SpaceInfo,
  SubscriptionStatus,
  TurnFinalized,
  TurnProgress,
} from "./types";

// 命令名保持 snake_case；参数名走 Tauri 默认 camelCase 转换。
export const cohub = {
  login: () => invoke<void>("login"),
  logout: () => invoke<void>("logout"),
  checkLogin: () => invoke<boolean>("check_login"),
  getAccount: () => invoke<AccountInfo | null>("get_account"),
  listSpaces: () => invoke<SpaceInfo[]>("list_spaces"),
  mountSpace: (spaceId: string, mountPath: string) =>
    invoke<void>("mount_space", { spaceId, mountPath }),
  unmountSpace: (spaceId: string) => invoke<void>("unmount_space", { spaceId }),
  listMounts: () => invoke<never[]>("list_mounts"),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  getSettings: () => invoke<AppSettings>("get_settings"),
  setSettings: (settings: AppSettings) =>
    invoke<void>("set_settings", { settings }),
  markFinalizedRead: () => invoke<void>("mark_finalized_read"),
};

export type EventMap = {
  "cohub-log": { message: string };
  "auth-status": AuthStatus;
  "spaces": { spaces: SpaceInfo[] };
  "subscription-status": SubscriptionStatus;
  "turn-progress": TurnProgress;
  "turn-finalized": TurnFinalized;
  "in-app-notify": {
    lastText?: string;
    messageCount: number;
    toolCallCount: number;
    hasError: boolean;
  };
  "settings-updated": AppSettings;
  "activity": Omit<ActivityItem, "ts">;
};

export function on<K extends keyof EventMap>(
  event: K,
  handler: (payload: EventMap[K]) => void,
): Promise<UnlistenFn> {
  return listen<EventMap[K]>(event, (e) => handler(e.payload));
}

/** 构造网页端 session 深链（与后端 session_url 一致）。 */
export function sessionUrl(
  spaceId?: string,
  sessionId?: string,
  sequence?: number,
): string | null {
  if (!spaceId || !sessionId) return null;
  const base = `https://cohub.run/spaces/${spaceId}/sessions/${sessionId}`;
  return sequence != null ? `${base}?turn=${sequence}` : base;
}
