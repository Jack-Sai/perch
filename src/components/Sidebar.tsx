import { Gauge, ListTree, Settings, SlidersHorizontal } from "lucide-react";
import { useI18n } from "../lib/i18n";
import { useStore } from "../lib/store";
import { cn } from "../lib/utils";
import { statusKind } from "../lib/types";

export type Page = "dashboard" | "parameters" | "profiles" | "settings";

const navItems: Array<{ id: Page; icon: typeof Gauge; labelKey: string }> = [
  { id: "dashboard", icon: Gauge, labelKey: "nav.dashboard" },
  { id: "parameters", icon: SlidersHorizontal, labelKey: "nav.parameters" },
  { id: "profiles", icon: ListTree, labelKey: "nav.profiles" },
  { id: "settings", icon: Settings, labelKey: "nav.settings" },
];

export function Sidebar({ page, onNavigate }: { page: Page; onNavigate: (p: Page) => void }) {
  const { t } = useI18n();
  const { cfg, editing, runningProfile, status } = useStore();

  return (
    <aside className="flex w-52 shrink-0 flex-col border-r border-border bg-card/40">
      <nav className="flex flex-1 flex-col gap-1 p-2.5">
        {navItems.map(({ id, icon: Icon, labelKey }) => {
          const active = page === id;
          return (
            <button
              key={id}
              onClick={() => onNavigate(id)}
              className={cn(
                "flex h-9 items-center gap-2.5 rounded-md px-3 text-sm transition-colors",
                active
                  ? "bg-primary/10 font-medium text-primary"
                  : "text-muted-foreground hover:bg-accent hover:text-foreground",
              )}
            >
              <Icon className="h-4 w-4" />
              {t(labelKey)}
              {id === "dashboard" && statusKind(status) === "running" && (
                <span className="ml-auto h-1.5 w-1.5 rounded-full bg-success" />
              )}
            </button>
          );
        })}
      </nav>

      <div className="border-t border-border p-3">
        <div className="text-[10px] uppercase tracking-wide text-muted-foreground">
          {t("dashboard.runningProfile")}
        </div>
        <div className="mt-0.5 truncate text-xs font-medium">
          {runningProfile ?? t("dashboard.none")}
        </div>
        <div className="mt-2 text-[10px] uppercase tracking-wide text-muted-foreground">
          {t("dashboard.editingProfile")}
        </div>
        <div className="mt-0.5 truncate text-xs">
          {editing || cfg?.profiles[0]?.name || t("dashboard.none")}
        </div>
      </div>
    </aside>
  );
}
