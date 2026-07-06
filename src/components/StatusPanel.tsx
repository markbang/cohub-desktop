import { useState } from "react";
import { Badge, Button, LayerCard, Switch, Text } from "@cloudflare/kumo";
import { Gear, SignOut } from "@phosphor-icons/react";
import { cohub } from "../lib/api";
import type { AccountInfo, AppSettings, SubscriptionStatus } from "../lib/types";

interface Props {
  account: AccountInfo;
  sub: SubscriptionStatus | null;
  settings: AppSettings;
  onLogout: () => void;
}

interface Row {
  key: keyof AppSettings;
  label: string;
  description: string;
}

const ROWS: Row[] = [
  {
    key: "only_my_spaces",
    label: "仅监听我创建的 Space",
    description: "过滤掉加入他人的 Space。",
  },
  {
    key: "only_my_turns",
    label: "仅监听我的对话",
    description: "只在自己发起的对话完结时通知。",
  },
  {
    key: "notify_finalized",
    label: "对话完结系统通知",
    description: "推送系统通知，可点击跳转网页端。",
  },
  {
    key: "tray_badge",
    label: "托盘未读红点",
    description: "有未查看的完结对话时显示计数。",
  },
  {
    key: "in_app_toast",
    label: "应用内 Toast",
    description: "窗口可见时弹出提示。",
  },
];

function subBadge(sub: SubscriptionStatus | null) {
  if (!sub) return <Badge variant="neutral">未连接</Badge>;
  switch (sub.phase) {
    case "connected":
      return (
        <Badge variant="success" appearance="dot">
          已连接 · {sub.space_count} 个 Space
        </Badge>
      );
    case "connecting":
      return (
        <Badge variant="warning" appearance="dot">
          连接中…
        </Badge>
      );
    case "reconnecting":
      return (
        <Badge variant="warning" appearance="dot">
          重连中…
        </Badge>
      );
    case "error":
      return (
        <Badge variant="error" appearance="dot">
          连接错误
        </Badge>
      );
    case "disconnected":
      return <Badge variant="neutral">已断开</Badge>;
    default:
      return <Badge variant="neutral">空闲</Badge>;
  }
}

export function StatusPanel({ account, sub, settings, onLogout }: Props) {
  const [draft, setDraft] = useState<AppSettings>(settings);
  const [saving, setSaving] = useState(false);

  // settings 变更（后端广播）时同步本地草稿。
  const [lastSettings, setLastSettings] = useState(settings);
  if (settings !== lastSettings) {
    setLastSettings(settings);
    setDraft(settings);
  }

  const update = (key: keyof AppSettings, value: boolean) => {
    const next = { ...draft, [key]: value };
    setDraft(next);
    setSaving(true);
    cohub
      .setSettings(next)
      .catch(() => {})
      .finally(() => setSaving(false));
  };

  return (
    <div className="flex h-full flex-col bg-kumo-canvas">
      <header className="flex shrink-0 items-center justify-between gap-3 px-5 py-4">
        <div className="flex items-center gap-2.5">
          <div className="flex size-7 items-center justify-center rounded-lg bg-kumo-brand text-[15px] font-bold text-white">
            c
          </div>
          <Text variant="heading3" as="span">
            Cohub Desktop
          </Text>
        </div>
        <Button
          variant="ghost"
          shape="square"
          size="sm"
          icon={<SignOut size={16} />}
          aria-label="登出"
          onClick={onLogout}
        />
      </header>

      <div className="flex-1 space-y-3 overflow-y-auto px-5 pb-5">
        <LayerCard className="rounded-xl p-4">
          <div className="flex items-center gap-3">
            <div className="flex size-10 shrink-0 items-center justify-center overflow-hidden rounded-full bg-kumo-fill">
              {account.avatar_url ? (
                <img
                  src={account.avatar_url}
                  alt={account.display_name}
                  className="size-10 rounded-full object-cover"
                />
              ) : (
                <span className="text-base font-semibold">
                  {account.display_name.slice(0, 1).toUpperCase()}
                </span>
              )}
            </div>
            <div className="min-w-0 flex-1">
              <Text variant="body" size="sm" bold truncate as="p">
                {account.display_name}
              </Text>
              <span className="block truncate font-mono text-xs text-kumo-subtle">
                {account.uuid.slice(0, 8)}
              </span>
            </div>
          </div>
          <div className="mt-3 flex items-center gap-2">
            {subBadge(sub)}
          </div>
        </LayerCard>

        <LayerCard className="rounded-xl p-4">
          <div className="mb-3 flex items-center gap-2">
            <Gear size={16} className="text-kumo-subtle" />
            <Text variant="heading3" as="h2">
              监听与通知
            </Text>
          </div>
          <div className="flex flex-col gap-1">
            {ROWS.map(({ key, label, description }) => (
              <div
                key={key}
                className="flex items-start justify-between gap-3 rounded-lg px-2 py-2.5"
              >
                <div className="min-w-0">
                  <Text variant="body" size="sm" bold as="p">
                    {label}
                  </Text>
                  <Text variant="secondary" size="xs" as="p">
                    {description}
                  </Text>
                </div>
                <Switch
                  checked={draft[key]}
                  onCheckedChange={(v) => update(key, v)}
                  disabled={saving}
                  size="sm"
                  aria-label={label}
                />
              </div>
            ))}
          </div>
        </LayerCard>
      </div>
    </div>
  );
}
