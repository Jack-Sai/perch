import type { LucideIcon } from "lucide-react";
import { cn } from "../lib/utils";

type Tone = "default" | "primary" | "success" | "warning" | "destructive" | "info";

const barColor: Record<Tone, string> = {
  default: "bg-primary",
  primary: "bg-primary",
  success: "bg-success",
  warning: "bg-warning",
  destructive: "bg-destructive",
  info: "bg-info",
};

const valueColor: Record<Tone, string> = {
  default: "text-foreground",
  primary: "text-foreground",
  success: "text-success",
  warning: "text-warning",
  destructive: "text-destructive",
  info: "text-info",
};

export function StatTile({
  icon: Icon,
  label,
  value,
  sub,
  percent,
  tone = "default",
  alert = false,
}: {
  icon?: LucideIcon;
  label: string;
  value: string;
  sub?: string;
  percent?: number;
  tone?: Tone;
  alert?: boolean;
}) {
  const showBar = percent != null && !Number.isNaN(percent);
  return (
    <div className="rounded-xl border border-border bg-card p-3 shadow-card">
      <div className="flex items-center justify-between">
        <span className="flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
          {Icon && <Icon className="h-3.5 w-3.5" />}
          {label}
        </span>
        {alert && (
          <span className="h-1.5 w-1.5 rounded-full bg-destructive animate-pulse" title="阈值告警" />
        )}
      </div>
      <div className={cn("mt-1.5 text-2xl font-semibold tabular leading-none", valueColor[tone])}>
        {value}
      </div>
      {sub && <div className="mt-1 truncate text-[11px] text-muted-foreground tabular">{sub}</div>}
      {showBar && (
        <div className="mt-2 h-1.5 w-full overflow-hidden rounded-full bg-muted">
          <div
            className={cn("h-full rounded-full transition-all duration-300", barColor[tone])}
            style={{ width: `${Math.max(0, Math.min(100, percent))}%` }}
          />
        </div>
      )}
    </div>
  );
}
