import { Toasty } from "@cloudflare/kumo";
import { cohub } from "./lib/api";
import { toastManager } from "./lib/toast";
import { useCohub } from "./state";
import { SignInCard } from "./components/SignInCard";
import { StatusPanel } from "./components/StatusPanel";
import type { AppSettings } from "./lib/types";

const DEFAULT_SETTINGS: AppSettings = {
  only_my_spaces: false,
  only_my_turns: false,
  notify_finalized: true,
  tray_badge: true,
  in_app_toast: true,
};

export default function App() {
  const state = useCohub();

  const loggedIn = state.auth.phase === "success";
  const account = state.auth.account ?? null;
  const settings = state.settings ?? DEFAULT_SETTINGS;

  return (
    <Toasty toastManager={toastManager}>
      <div className="h-full text-kumo-default">
        {loggedIn && account ? (
          <StatusPanel
            account={account}
            sub={state.sub}
            settings={settings}
            onLogout={() => cohub.logout().catch(() => {})}
          />
        ) : (
          <SignInCard auth={state.auth} onLogin={() => cohub.login().catch(() => {})} />
        )}
      </div>
    </Toasty>
  );
}
