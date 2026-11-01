import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Moon, Square, Sun, X, Languages } from "lucide-react";
import { useI18n } from "../lib/i18n";
import { useTheme } from "../lib/theme";
import { useStore } from "../lib/store";
import { StatusBadge } from "./StatusBadge";
import { cn } from "../lib/utils";

export function TitleBar() {
  const { t, lang, setLang } = useI18n();
  const { resolved, setTheme } = useTheme();
  const { status, cfg, saveCfg } = useStore();
  const win = getCurrentWindow();

  const toggleTheme = () => {
    const next = resolved === "dark" ? "light" : "dark";
    setTheme(next);
    if (cfg) {
      const nextCfg = { ...cfg, global: { ...cfg.global, theme: next } };
      void saveCfg(nextCfg, { silent: true });
    }
  };

  const toggleLang = () => {
    const next = lang === "zh-CN" ? "en-US" : "zh-CN";
    setLang(next);
    if (cfg) {
      const nextCfg = { ...cfg, global: { ...cfg.global, language: next } };
      void saveCfg(nextCfg, { silent: true });
    }
  };

  return (
    <header
      data-tauri-drag-region
      className="flex h-11 shrink-0 select-none items-center justify-between border-b border-border bg-card pl-4 pr-0"
    >
      <div className="flex items-center gap-2.5" data-tauri-drag-region>
        <div className="flex h-6 w-6 items-center justify-center rounded-md bg-primary text-primary-foreground">
          <svg viewBox="0 0 24 24" className="h-3.5 w-3.5" fill="none" stroke="currentColor" strokeWidth="2.2">
            <path d="M4 16c3-1 5-6 8-6s5 3 8 2" strokeLinecap="round" />
            <path d="M7 19c2.5-.8 4-4 7-4" strokeLinecap="round" opacity="0.6" />
          </svg>
        </div>
        <span className="text-sm font-semibold tracking-tight">{t("app.name")}</span>
        <span className="hidden text-[11px] text-muted-foreground md:inline">{t("app.tagline")}</span>
        <StatusBadge status={status} />
      </div>

      <div className="flex items-center">
        <button
          onClick={toggleLang}
          title={lang === "zh-CN" ? "English" : "中文"}
          className="flex h-11 w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
        >
          <Languages className="h-4 w-4" />
        </button>
        <button
          onClick={toggleTheme}
          title={resolved === "dark" ? "Light" : "Dark"}
          className="flex h-11 w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
        >
          {resolved === "dark" ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
        </button>
        <div className="mx-1 h-4 w-px bg-border" />
        <button
          onClick={() => void win.minimize()}
          title={t("titlebar.minimize")}
          className={cn(
            "flex h-11 w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-accent hover:text-foreground",
          )}
        >
          <Minus className="h-4 w-4" />
        </button>
        <button
          onClick={() => void win.toggleMaximize()}
          title={t("titlebar.maximize")}
          className="flex h-11 w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
        >
          <Square className="h-3.5 w-3.5" />
        </button>
        <button
          onClick={() => void win.close()}
          title={t("titlebar.close")}
          className="flex h-11 w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-destructive hover:text-destructive-foreground"
        >
          <X className="h-4 w-4" />
        </button>
      </div>
    </header>
  );
}
