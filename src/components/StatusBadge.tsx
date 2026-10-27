import { useI18n } from "../lib/i18n";
import { statusKind, statusError, type ServerStatus } from "../lib/types";
import { cn } from "../lib/utils";

const dotColor: Record<string, string> = {
  idle: "bg-muted-foreground",
  launching: "bg-warning animate-pulse",
  running: "bg-success",
  stopping: "bg-warning animate-pulse",
  error: "bg-destructive",
};

export function StatusBadge({ status, showText = true }: { status: ServerStatus; showText?: boolean }) {
  const { t } = useI18n();
  const kind = statusKind(status);
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full border border-border bg-card px-2.5 py-1 text-[11px] font-medium",
        kind === "running" && "border-success/30",
        kind === "error" && "border-destructive/30",
      )}
      title={statusError(status)}
    >
      <span className={cn("h-1.5 w-1.5 rounded-full", dotColor[kind])} />
      {showText && t(`status.${kind}`)}
    </span>
  );
}
