import { Badge, Button, Empty, Text } from "@cloudflare/kumo";
import { ArrowsClockwise } from "@phosphor-icons/react";
import { cohub } from "../lib/api";
import type { ActivityItem, SpaceInfo, SubscriptionStatus } from "../lib/types";
import type { TurnView } from "../state";
import { ActivityFeed } from "./ActivityFeed";
import { TurnCard } from "./TurnCard";

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

interface Props {
  spaces: SpaceInfo[];
  sub: SubscriptionStatus | null;
  turns: Record<string, TurnView>;
  activityItems: ActivityItem[];
}

export function LiveActivity({ spaces, sub, turns, activityItems }: Props) {
  const spaceName = (id?: string) =>
    (id && spaces.find((s) => s.id === id)?.name) || undefined;

  const list = Object.values(turns).sort((a, b) => {
    if (a.finalized !== b.finalized) return a.finalized ? 1 : -1;
    return b.updatedAt - a.updatedAt;
  });

  return (
    <div className="flex h-full min-h-0 flex-col">
      <header className="flex shrink-0 items-center justify-between gap-4 px-6 py-5">
        <div>
          <Text variant="heading2" as="h1">
            实时活动
          </Text>
          <Text variant="secondary" size="sm">
            监听你所有 Space 的对话流式状态，完结时推送通知。
          </Text>
        </div>
        <div className="flex items-center gap-2">
          {subBadge(sub)}
          <Button
            variant="ghost"
            shape="square"
            size="sm"
            icon={<ArrowsClockwise size={16} />}
            aria-label="刷新 Space 列表"
            onClick={() => cohub.listSpaces().catch(() => {})}
          />
        </div>
      </header>

      <div className="grid min-h-0 flex-1 grid-cols-1 gap-5 px-6 pb-6 lg:grid-cols-[minmax(0,2fr)_minmax(0,1fr)]">
        <section className="flex min-h-0 flex-col gap-3">
          <div className="flex items-baseline justify-between">
            <Text variant="heading3" as="h2">
              对话
            </Text>
            <Text variant="secondary" size="xs">
              {list.length} 个
            </Text>
          </div>
          <div className="min-h-0 flex-1 space-y-3 overflow-y-auto pr-1">
            {list.length === 0 ? (
              <Empty
                title="还没有对话事件"
                description="当 Space 里的对话开始生成时会出现在这里。"
              />
            ) : (
              list.map((t) => (
                <TurnCard
                  key={t.turnId ?? t.sessionId ?? Math.random()}
                  turn={t}
                  spaceName={spaceName(t.spaceId)}
                />
              ))
            )}
          </div>
        </section>

        <ActivityFeed items={activityItems} />
      </div>
    </div>
  );
}
