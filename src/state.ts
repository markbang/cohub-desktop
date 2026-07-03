import { useEffect, useReducer } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { cohub, on } from "./lib/api";
import { toastManager } from "./lib/toast";
import type {
  ActivityItem,
  AppSettings,
  AuthStatus,
  SpaceInfo,
  SubscriptionStatus,
  TurnFinalized,
  TurnProgress,
} from "./lib/types";

/** 单个对话的合并视图：流式进度 + 完结快照。 */
export interface TurnView {
  sessionId?: string;
  spaceId?: string;
  turnId?: string;
  // 流式进度
  llmRound?: number;
  provider?: string;
  model?: string;
  steps: number;
  tools: number;
  // 完结快照（finalized 后填充）
  finalized: boolean;
  status?: string;
  sequence?: number;
  messageCount?: number;
  toolCallCount?: number;
  usageInput?: number;
  usageOutput?: number;
  usageCacheRead?: number;
  durationMs?: number;
  lastText?: string;
  hasError?: boolean;
  updatedAt: number;
}

export interface AppState {
  auth: AuthStatus;
  spaces: SpaceInfo[];
  sub: SubscriptionStatus | null;
  turns: Record<string, TurnView>;
  activity: ActivityItem[];
  settings: AppSettings | null;
}

export const initialState: AppState = {
  auth: { phase: "idle" },
  spaces: [],
  sub: null,
  turns: {},
  activity: [],
  settings: null,
};

type Action =
  | { type: "auth-status"; payload: AuthStatus }
  | { type: "spaces"; payload: SpaceInfo[] }
  | { type: "subscription-status"; payload: SubscriptionStatus }
  | { type: "turn-progress"; payload: TurnProgress }
  | { type: "turn-finalized"; payload: TurnFinalized }
  | { type: "activity"; payload: Omit<ActivityItem, "ts"> }
  | { type: "settings"; payload: AppSettings };

function turnKey(p: { session_id?: string; turn_id?: string }): string {
  return p.turn_id ?? p.session_id ?? "unknown";
}

export function reducer(state: AppState, action: Action): AppState {
  switch (action.type) {
    case "auth-status":
      return { ...state, auth: action.payload };
    case "spaces":
      return { ...state, spaces: action.payload };
    case "subscription-status":
      return { ...state, sub: action.payload };
    case "turn-progress": {
      const key = turnKey(action.payload);
      const prev = state.turns[key];
      const next: TurnView = {
        sessionId: action.payload.session_id ?? prev?.sessionId,
        spaceId: action.payload.space_id ?? prev?.spaceId,
        turnId: action.payload.turn_id ?? prev?.turnId,
        llmRound: action.payload.llm_round ?? prev?.llmRound,
        provider: action.payload.provider ?? prev?.provider,
        model: action.payload.model ?? prev?.model,
        steps: action.payload.steps,
        tools: action.payload.tools,
        finalized: prev?.finalized ?? false,
        status: prev?.status,
        sequence: prev?.sequence,
        messageCount: prev?.messageCount,
        toolCallCount: prev?.toolCallCount,
        usageInput: prev?.usageInput,
        usageOutput: prev?.usageOutput,
        usageCacheRead: prev?.usageCacheRead,
        durationMs: prev?.durationMs,
        lastText: prev?.lastText,
        hasError: prev?.hasError,
        updatedAt: Date.now(),
      };
      return { ...state, turns: { ...state.turns, [key]: next } };
    }
    case "turn-finalized": {
      const key = turnKey(action.payload);
      const prev = state.turns[key];
      const next: TurnView = {
        sessionId: action.payload.session_id ?? prev?.sessionId,
        spaceId: action.payload.space_id ?? prev?.spaceId,
        turnId: action.payload.turn_id ?? prev?.turnId,
        llmRound: prev?.llmRound,
        provider: action.payload.provider ?? prev?.provider,
        model: action.payload.model ?? prev?.model,
        steps: action.payload.message_count || prev?.steps || 0,
        tools: action.payload.tool_call_count || prev?.tools || 0,
        finalized: true,
        status: action.payload.status,
        sequence: action.payload.sequence,
        messageCount: action.payload.message_count,
        toolCallCount: action.payload.tool_call_count,
        usageInput: action.payload.usage_input,
        usageOutput: action.payload.usage_output,
        usageCacheRead: action.payload.usage_cache_read,
        durationMs: action.payload.duration_ms,
        lastText: action.payload.last_text,
        hasError: action.payload.has_error,
        updatedAt: Date.now(),
      };
      return { ...state, turns: { ...state.turns, [key]: next } };
    }
    case "activity": {
      const item: ActivityItem = { ...action.payload, ts: Date.now() };
      return { ...state, activity: [item, ...state.activity].slice(0, 120) };
    }
    case "settings":
      return { ...state, settings: action.payload };
    default:
      return state;
  }
}

/** 订阅全部后端事件 + 启动时同步登录态。 */
export function useCohub() {
  const [state, dispatch] = useReducer(reducer, initialState);

  useEffect(() => {
    const unlistens: UnlistenFn[] = [];
    let cancelled = false;

    (async () => {
      unlistens.push(
        await on("auth-status", (p) => dispatch({ type: "auth-status", payload: p })),
      );
      unlistens.push(
        await on("spaces", (p) => dispatch({ type: "spaces", payload: p.spaces })),
      );
      unlistens.push(
        await on("subscription-status", (p) =>
          dispatch({ type: "subscription-status", payload: p }),
        ),
      );
      unlistens.push(
        await on("activity", (p) => dispatch({ type: "activity", payload: p })),
      );
      unlistens.push(
        await on("turn-progress", (p) => dispatch({ type: "turn-progress", payload: p })),
      );
      unlistens.push(
        await on("turn-finalized", (p) => {
          dispatch({ type: "turn-finalized", payload: p });
        }),
      );
      unlistens.push(
        await on("in-app-notify", (p) => {
          const title = p.hasError ? "对话出错" : "对话完成";
          const desc = p.lastText
            ? truncate(p.lastText, 80)
            : `${p.messageCount} steps · ${p.toolCallCount} tools`;
          toastManager.add({
            title,
            description: desc,
            variant: p.hasError ? "error" : "success",
          });
        }),
      );
      unlistens.push(
        await on("settings-updated", (p) => dispatch({ type: "settings", payload: p })),
      );

      if (cancelled) return;
      // 加载初始设置。
      try {
        const s = await cohub.getSettings();
        dispatch({ type: "settings", payload: s });
      } catch {}
      const account = await cohub.getAccount();
      if (account) {
        dispatch({ type: "auth-status", payload: { phase: "success", account } });
      } else {
        const loggedIn = await cohub.checkLogin();
        dispatch({
          type: "auth-status",
          payload: { phase: loggedIn ? "restoring" : "logged_out" },
        });
      }
    })();

    return () => {
      cancelled = true;
      unlistens.forEach((u) => u());
    };
  }, []);

  return state;
}

function truncate(s: string, max: number): string {
  const t = s.trim();
  if ([...t].length <= max) return t;
  return [...t].slice(0, max).join("") + "…";
}
