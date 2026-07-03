import { useState } from "react";
import { Toasty } from "@cloudflare/kumo";
import { cohub } from "./lib/api";
import { toastManager } from "./lib/toast";
import { useCohub } from "./state";
import { CloudDrive } from "./components/CloudDrive";
import { LiveActivity } from "./components/LiveActivity";
import { SettingsDialog } from "./components/SettingsDialog";
import { Sidebar, type View } from "./components/Sidebar";
import { SignInCard } from "./components/SignInCard";
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
  const [view, setView] = useState<View>("activity");
  const [settingsOpen, setSettingsOpen] = useState(false);

  const loggedIn = state.auth.phase === "success";
  const account = state.auth.account ?? null;
  const settings = state.settings ?? DEFAULT_SETTINGS;

  const startLogin = () => {
    setView("activity");
    cohub.login().catch(() => {});
  };

  let main: React.ReactNode;
  if (view === "drive") {
    main = <CloudDrive loggedIn={loggedIn} spaces={state.spaces} />;
  } else if (!loggedIn) {
    main = <SignInCard auth={state.auth} onLogin={startLogin} />;
  } else {
    main = (
      <LiveActivity
        spaces={state.spaces}
        sub={state.sub}
        turns={state.turns}
        activityItems={state.activity}
      />
    );
  }

  return (
    <Toasty toastManager={toastManager}>
      <div className="flex h-full bg-kumo-canvas text-kumo-default">
        <Sidebar
          view={view}
          onView={setView}
          phase={state.auth.phase}
          account={account}
          onLogin={startLogin}
          onLogout={() => cohub.logout().catch(() => {})}
          onOpenSettings={() => setSettingsOpen(true)}
        />
        <main className="min-w-0 flex-1">{main}</main>
      </div>
      <SettingsDialog
        open={settingsOpen}
        onOpenChange={setSettingsOpen}
        settings={settings}
      />
    </Toasty>
  );
}
