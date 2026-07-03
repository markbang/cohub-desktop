// 与 src-tauri/src/ws.rs 的 TurnProgress / TurnFinalized 字段一一对应（snake_case）。

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

export interface SpaceInfo {
  id: string;
  name: string;
  user_uuid?: string;
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

/** 进行中的对话进度（session.turn.lifecycle + session.turn.patch 累积）。 */
export interface TurnProgress {
  space_id?: string;
  session_id?: string;
  turn_id?: string;
  llm_round?: number;
  provider?: string;
  model?: string;
  steps: number;
  tools: number;
}

/** 对话完结快照（session.turn.finalized）。 */
export interface TurnFinalized {
  space_id?: string;
  session_id?: string;
  turn_id?: string;
  sequence?: number;
  status?: string;
  provider?: string;
  model?: string;
  message_count: number;
  tool_call_count: number;
  usage_input: number;
  usage_output: number;
  usage_cache_read: number;
  duration_ms?: number;
  last_text?: string;
  has_error: boolean;
  user_uuid?: string;
}

export interface ActivityItem {
  eventType: string;
  spaceId?: string;
  sessionId?: string;
  ts: number;
}

export interface AppSettings {
  only_my_spaces: boolean;
  only_my_turns: boolean;
  notify_finalized: boolean;
  tray_badge: boolean;
  in_app_toast: boolean;
}
