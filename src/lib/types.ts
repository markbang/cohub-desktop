// 与 src-tauri 的事件 payload / 命令返回值一一对应（snake_case）。

export type AuthPhase =
  | "idle"
  | "requesting_device"
  | "awaiting_user"
  | "polling"
  | "success"
  | "error"
  | "logged_out"
  | "restoring";

export interface AccountInfo {
  uuid: string;
  display_name: string;
  avatar_url?: string;
}

export interface AuthStatus {
  phase: AuthPhase;
  user_code?: string;
  verification_uri?: string;
  verification_uri_complete?: string;
  account?: AccountInfo;
  error?: string;
}

export type SubscriptionPhase =
  | "idle"
  | "connecting"
  | "connected"
  | "reconnecting"
  | "error"
  | "disconnected";

export interface SubscriptionStatus {
  phase: SubscriptionPhase;
  space_count: number;
  error?: string;
}

export interface AppSettings {
  only_my_spaces: boolean;
  only_my_turns: boolean;
  notify_finalized: boolean;
  tray_badge: boolean;
  in_app_toast: boolean;
}
