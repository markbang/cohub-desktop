import { Text } from "@cloudflare/kumo";
import type { ActivityItem } from "../lib/types";

function clock(ts: number): string {
  const d = new Date(ts);
  const p = (n: number) => n.toString().padStart(2, "0");
  return `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}

interface Props {
  items: ActivityItem[];
}

export function ActivityFeed({ items }: Props) {
  return (
    <div className="flex h-full min-h-0 flex-col rounded-xl border border-kumo-line bg-kumo-base">
      <div className="border-b border-kumo-line px-4 py-3">
        <Text variant="heading3" as="h3">
          活动流
        </Text>
        <Text variant="secondary" size="xs">
          最近 {items.length} 条原始事件
        </Text>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto">
        {items.length === 0 ? (
          <div className="px-4 py-8 text-center">
            <Text variant="secondary" size="sm">
              暂无活动，等待 Space 事件…
            </Text>
          </div>
        ) : (
          <ul className="divide-y divide-kumo-line">
            {items.map((it, i) => (
              <li key={i} className="flex items-center gap-3 px-4 py-2">
                <time className="shrink-0 font-mono text-xs text-kumo-subtle">
                  {clock(it.ts)}
                </time>
                <span className="shrink-0 font-mono text-xs text-kumo-default">
                  {it.eventType}
                </span>
                <span className="truncate font-mono text-xs text-kumo-subtle">
                  {it.sessionId ?? ""}
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
