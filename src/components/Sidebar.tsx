import { Button, Text } from "@cloudflare/kumo";
import {
  Gear,
  HardDrives,
  Pulse,
  SignIn,
  SignOut,
} from "@phosphor-icons/react";
import type { AccountInfo, AuthPhase } from "../lib/types";

export type View = "activity" | "drive";

interface Props {
  view: View;
  onView: (v: View) => void;
  phase: AuthPhase;
  account: AccountInfo | null;
  onLogin: () => void;
  onLogout: () => void;
  onOpenSettings: () => void;
}

const NAV: { id: View; label: string; icon: typeof Pulse }[] = [
  { id: "activity", label: "实时活动", icon: Pulse },
  { id: "drive", label: "云盘挂载", icon: HardDrives },
];

export function Sidebar({ view, onView, phase, account, onLogin, onLogout, onOpenSettings }: Props) {
  const loggedIn = phase === "success";
  return (
    <aside className="flex h-full w-[224px] shrink-0 flex-col border-r border-kumo-line bg-kumo-base">
      <div className="flex items-center gap-2.5 px-4 py-4">
        <div className="flex size-7 items-center justify-center rounded-lg bg-kumo-brand text-[15px] font-bold text-white">
          c
        </div>
        <div className="leading-tight">
          <Text variant="heading3" as="span">
            cohub
          </Text>
          <Text variant="secondary" size="xs" as="span">
            {" "}
            桌面伴侣
          </Text>
        </div>
      </div>

      <nav className="mt-2 flex flex-col gap-1 px-2">
        {NAV.map(({ id, label, icon: Icon }) => {
          const active = view === id;
          return (
            <Button
              key={id}
              variant="ghost"
              icon={<Icon size={18} weight="regular" />}
              onClick={() => onView(id)}
              className={`w-full justify-start ${
                active ? "bg-kumo-tint text-kumo-strong" : "text-kumo-subtle"
              }`}
            >
              {label}
            </Button>
          );
        })}
      </nav>

      <div className="mt-auto border-t border-kumo-line p-3">
        {loggedIn && account ? (
          <div className="flex items-center gap-2">
            <div className="flex size-8 shrink-0 items-center justify-center overflow-hidden rounded-full bg-kumo-fill text-kumo-default">
              {account.avatar_url ? (
                <img
                  src={account.avatar_url}
                  alt={account.display_name}
                  className="size-8 rounded-full object-cover"
                />
              ) : (
                <span className="text-sm font-semibold">
                  {account.display_name.slice(0, 1).toUpperCase()}
                </span>
              )}
            </div>
            <div className="min-w-0 flex-1">
              <Text variant="body" size="sm" truncate as="p">
                {account.display_name}
              </Text>
              <span className="block truncate font-mono text-xs text-kumo-subtle">
                {account.uuid.slice(0, 8)}
              </span>
            </div>
            <Button
              variant="ghost"
              shape="square"
              size="sm"
              icon={<Gear size={16} weight="regular" />}
              aria-label="应用设置"
              onClick={onOpenSettings}
            />
            <Button
              variant="ghost"
              shape="square"
              size="sm"
              icon={<SignOut size={16} />}
              aria-label="登出"
              onClick={onLogout}
            />
          </div>
        ) : (
          <Button
            variant="secondary"
            icon={<SignIn size={16} />}
            className="w-full"
            onClick={onLogin}
          >
            登录 Cohub
          </Button>
        )}
      </div>
    </aside>
  );
}
