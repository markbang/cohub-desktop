import { useState } from "react";
import {
  Button,
  Dialog,
  DialogDescription,
  DialogRoot,
  DialogTitle,
  Switch,
  Text,
} from "@cloudflare/kumo";
import { cohub } from "../lib/api";
import type { AppSettings } from "../lib/types";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  settings: AppSettings;
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
    description: "只接收自己拥有的 Space 的事件，过滤掉加入他人的 Space。",
  },
  {
    key: "only_my_turns",
    label: "仅监听我的对话",
    description: "只在自己发起的对话完结时通知，忽略他人在同一 Space 的对话。",
  },
  {
    key: "notify_finalized",
    label: "对话完结系统通知",
    description: "对话结束时推送系统通知（可点击跳转到网页端）。",
  },
  {
    key: "tray_badge",
    label: "托盘未读红点",
    description: "有未查看的完结对话时，在托盘图标显示未读计数。",
  },
  {
    key: "in_app_toast",
    label: "应用内 Toast",
    description: "对话完结时在应用内弹出 Toast 提示。",
  },
];

export function SettingsDialog({ open, onOpenChange, settings }: Props) {
  const [draft, setDraft] = useState<AppSettings>(settings);
  const [saving, setSaving] = useState(false);

  // 对话框每次打开时同步最新设置。
  const [lastOpen, setLastOpen] = useState(open);
  if (open !== lastOpen) {
    setLastOpen(open);
    if (open) setDraft(settings);
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
    <DialogRoot
      open={open}
      onOpenChange={(v) => onOpenChange(v)}
    >
      <Dialog className="p-6" size="base">
        <DialogTitle>
          <Text variant="heading2" as="span">
            应用设置
          </Text>
        </DialogTitle>
        <DialogDescription>
          <Text variant="secondary" size="sm" as="p">
            实时活动监听与通知偏好。更改即时生效并自动保存。
          </Text>
        </DialogDescription>

        <div className="mt-5 flex flex-col gap-1">
          {ROWS.map(({ key, label, description }) => (
            <div
              key={key}
              className="flex items-start justify-between gap-4 rounded-lg px-3 py-3 hover:bg-kumo-tint/50"
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

        <div className="mt-6 flex justify-end">
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
            完成
          </Button>
        </div>
      </Dialog>
    </DialogRoot>
  );
}
