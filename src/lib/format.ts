// 对齐 ~/code/cohub/apps/web 的 formatTokenCount / formatDurationMs / usageBreakdown。

export function formatTokenCount(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return `${n}`;
}

export function formatDurationMs(ms: number): string {
  if (ms < 1000) return `${Math.max(1, Math.round(ms))}ms`;
  if (ms < 10_000) {
    const seconds = Math.round(ms / 100) / 10;
    if (seconds < 10) return `${seconds.toFixed(1)}s`;
  }
  const totalSeconds = Math.round(ms / 1000);
  if (totalSeconds < 60) return `${totalSeconds}s`;
  const totalMinutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (totalMinutes < 60)
    return seconds > 0 ? `${totalMinutes}m ${seconds}s` : `${totalMinutes}m`;
  const hours = Math.floor(totalMinutes / 60);
  const remainingMinutes = totalMinutes % 60;
  return remainingMinutes > 0 ? `${hours}h ${remainingMinutes}m` : `${hours}h`;
}

/** web 风格的 usage breakdown：↑3.4M (3.4M cached) ↓1.3k */
export function usageBreakdown(
  input: number,
  output: number,
  cached: number,
): string {
  const inputTotal = input + cached;
  const parts: string[] = [];
  if (inputTotal > 0) parts.push(`↑${formatTokenCount(inputTotal)}`);
  if (cached > 0) parts.push(`(${formatTokenCount(cached)} cached)`);
  if (output > 0) parts.push(`↓${formatTokenCount(output)}`);
  return parts.join(" ");
}

export function truncatePreview(text: string, max: number): string {
  const t = text.trim();
  const chars = [...t];
  if (chars.length <= max) return t;
  return chars.slice(0, max).join("") + "…";
}
