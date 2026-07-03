import { Badge, Button, LayerCard, Text } from "@cloudflare/kumo";
import { ArrowUpRight } from "@phosphor-icons/react";
import { cohub, sessionUrl } from "../lib/api";
import { formatDurationMs, truncatePreview, usageBreakdown } from "../lib/format";
import type { TurnView } from "../state";

interface Props {
  turn: TurnView;
  spaceName?: string;
}

function plural(n: number, word: string): string {
  return `${n} ${word}${n === 1 ? "" : "s"}`;
}

export function TurnCard({ turn, spaceName }: Props) {
  const url = sessionUrl(turn.spaceId, turn.sessionId, turn.sequence);
  const modelName = turn.model;

  // 完结：展示 web 风格的完整 summary。
  const summaryParts: string[] = [];
  if (turn.finalized) {
    const steps = turn.messageCount ?? turn.steps;
    const tools = turn.toolCallCount ?? turn.tools;
    if (steps > 0) summaryParts.push(plural(steps, "step"));
    if (tools > 0) summaryParts.push(plural(tools, "tool"));
    const usage = usageBreakdown(
      turn.usageInput ?? 0,
      turn.usageOutput ?? 0,
      turn.usageCacheRead ?? 0,
    );
    if (usage) summaryParts.push(usage);
    if (turn.durationMs != null && turn.durationMs > 0)
      summaryParts.push(formatDurationMs(turn.durationMs));
  } else {
    // 进行中：steps / tools（live 累积）+ model。
    if (turn.steps > 0) summaryParts.push(plural(turn.steps, "step"));
    if (turn.tools > 0) summaryParts.push(plural(turn.tools, "tool"));
    if (modelName) summaryParts.push(modelName);
  }
  const summary = summaryParts.join(" · ") || (turn.finalized ? "完成" : "启动中");

  return (
    <LayerCard className="rounded-xl p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <Text variant="body" size="sm" bold truncate as="p">
              {spaceName ?? `session ${turn.sessionId?.slice(0, 8) ?? "—"}`}
            </Text>
            {turn.finalized ? (
              turn.hasError ? (
                <Badge variant="error" appearance="dot">
                  出错
                </Badge>
              ) : (
                <Badge variant="success" appearance="dot">
                  已完成
                </Badge>
              )
            ) : (
              <span className="inline-flex shrink-0 items-center gap-1.5 text-kumo-info">
                <span className="live-dot size-1.5 rounded-full bg-kumo-info" />
                <span className="font-mono text-xs">生成中</span>
              </span>
            )}
          </div>
          <span className="block truncate font-mono text-xs text-kumo-subtle">
            {turn.sessionId ?? "—"}
          </span>
        </div>
        {url && (
          <Button
            variant="ghost"
            shape="square"
            size="sm"
            icon={<ArrowUpRight size={15} weight="regular" />}
            aria-label="在网页打开"
            onClick={() => cohub.openUrl(url)}
          />
        )}
      </div>

      <div className="mt-2.5 flex flex-wrap items-center gap-x-2 gap-y-1">
        <span className="font-mono text-xs tabular-nums text-kumo-default">
          {summary}
        </span>
        {turn.finalized && turn.provider && (
          <span className="text-xs text-kumo-subtle">{turn.provider}</span>
        )}
      </div>

      {turn.finalized && turn.lastText && (
        <p className="mt-2 line-clamp-2 text-xs leading-relaxed text-kumo-subtle">
          {truncatePreview(turn.lastText, 160)}
        </p>
      )}
    </LayerCard>
  );
}
