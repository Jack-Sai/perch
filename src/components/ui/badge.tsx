import React from "react";
import { cn } from "../../lib/utils";

type Tone = "default" | "success" | "warning" | "destructive" | "muted" | "outline";

const tones: Record<Tone, string> = {
  default: "bg-primary/12 text-primary border-primary/20",
  success: "bg-success/12 text-success border-success/25",
  warning: "bg-warning/15 text-warning border-warning/30",
  destructive: "bg-destructive/12 text-destructive border-destructive/25",
  muted: "bg-muted text-muted-foreground border-border",
  outline: "bg-transparent text-foreground border-border",
};

export function Badge({
  tone = "default",
  className,
  ...props
}: React.HTMLAttributes<HTMLSpanElement> & { tone?: Tone }) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] font-medium leading-none",
        tones[tone],
        className,
      )}
      {...props}
    />
  );
}
