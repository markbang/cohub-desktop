import { useEffect, useReducer } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { cohub, on } from "./lib/api";
import { toastManager } from "./lib/toast";
import type { AppSettings, AuthStatus, SubscriptionStatus } from "./lib/types";

export interface AppState {
  auth: AuthStatus;
  sub: SubscriptionStatus | null;
  settings: AppSettings | null;
}

export const initialState: AppState = {
  auth: { phase: "idle" },
  sub: null,
  settings: null,
};

type Action =
  | { type: "auth-status"; payload: AuthStatus }
  | { type: "subscription-status"; payload: SubscriptionStatus }
  | { type: "settings"; payload: AppSettings };

export function reducer(state: AppState, action: Action): AppState {
  switch (action.type) {
    case "auth-status":
      return { ...state, auth: action.payload };
    case "subscription-status":
      return { ...state, sub: action.payload };
    case "settings":
      return { ...state, settings: action.payload };
    default:
      return state;
  }
}

/** 订阅后端事件 + 启动时同步登录态与设置。 */
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
        await on("subscription-status", (p) =>
          dispatch({ type: "subscription-status", payload: p }),
        ),
      );
      unlistens.push(
        await on("settings-updated", (p) => dispatch({ type: "settings", payload: p })),
      );
      // 应用内 Toast（对话完结时，窗口可见才显示）。
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

      if (cancelled) return;
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
  const chars = [...t];
  if (chars.length <= max) return t;
  return chars.slice(0, max).join("") + "…";
}
