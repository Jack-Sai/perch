import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Check,
  ChevronLeft,
  ChevronRight,
  Cpu,
  FolderOpen,
  Gauge,
  HardDrive,
  Languages,
  Loader2,
  Server,
  Sparkles,
} from "lucide-react";
import { api } from "../lib/api";
import { useI18n, type Lang } from "../lib/i18n";
import { useStore } from "../lib/store";
import { useTheme, type Theme } from "../lib/theme";
import { useToast } from "../lib/toast";
import { cn } from "../lib/utils";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { Select } from "./ui/select";

const STEP_KEYS = [
  "wizard.step.binary",
  "wizard.step.model",
  "wizard.step.port",
  "wizard.step.appearance",
  "wizard.step.monitor",
];

const STEP_ICONS = [Cpu, HardDrive, Server, Languages, Gauge];

export function Wizard() {
  const { t, lang, setLang } = useI18n();
  const { theme, setTheme } = useTheme();
  const {
    cfg,
    wizardOpen,
    setWizardOpen,
    setBinary,
    binary,
    editingProfile,
    updateProfile,
    updateGlobal,
    saveCfg,
  } = useStore();
  const { toast } = useToast();

  const [step, setStep] = useState(0);
  const [detecting, setDetecting] = useState(false);
  const [detectFailed, setDetectFailed] = useState(false);
  const [portMsg, setPortMsg] = useState<{ ok: boolean; text: string } | null>(null);
  const [freePort, setFreePort] = useState<number | null>(null);

  if (!wizardOpen || !cfg) return null;

  const total = STEP_KEYS.length;

  const autoDetect = async () => {
    setDetecting(true);
    setDetectFailed(false);
    try {
      const info = await api.detectBinary();
      setBinary(info);
      updateGlobal({ binary_path: info.path });
      toast(`${info.backend} · ${info.version}`, "success");
    } catch {
      setDetectFailed(true);
    } finally {
      setDetecting(false);
    }
  };

  const choose = async (directory: boolean) => {
    const picked = await open({
      multiple: false,
      directory,
      filters: directory ? undefined : [{ name: "llama-server", extensions: ["exe"] }],
    });
    if (typeof picked !== "string") return;
    try {
      const info = await api.resolveBinary(picked);
      setBinary(info);
      updateGlobal({ binary_path: info.path });
      setDetectFailed(false);
      toast(`${info.backend} · ${info.version}`, "success");
    } catch (e) {
      toast(String(e), "error");
    }
  };

  const chooseModel = async () => {
    const picked = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "GGUF", extensions: ["gguf"] }],
    });
    if (typeof picked === "string" && editingProfile) {
      updateProfile(editingProfile.name, { model_path: picked });
    }
  };

  const checkPort = async () => {
    if (!editingProfile) return;
    try {
      const st = await api.checkPort(editingProfile.port);
      if (st.available) {
        setPortMsg({ ok: true, text: t("wizard.portFree") });
        setFreePort(null);
      } else {
        setPortMsg({
          ok: false,
          text: t("wizard.portTaken", { name: st.process_name ?? "?" }),
        });
        try {
          setFreePort(await api.findFreePort(Math.max(editingProfile.port + 1, 1024)));
        } catch {
          setFreePort(null);
        }
      }
    } catch (e) {
      toast(String(e), "error");
    }
  };

  const finish = async (skipped = false) => {
    const next = { ...cfg, global: { ...cfg.global, first_run_done: true } };
    try {
      await saveCfg(next, { silent: true });
      setWizardOpen(false);
      if (!skipped) toast(t("wizard.stepDone"), "success");
    } catch {
      /* toast shown */
    }
  };

  const Icon = STEP_ICONS[step];

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center">
      <div className="absolute inset-0 bg-background/80 backdrop-blur-sm" />
      <div className="relative z-10 flex w-full max-w-2xl animate-fade-in overflow-hidden rounded-2xl border border-border bg-card shadow-2xl">
        {/* 左侧步骤 */}
        <div className="hidden w-48 shrink-0 flex-col gap-1 border-r border-border bg-muted/30 p-3 sm:flex">
          <div className="mb-3 flex items-center gap-2 px-2 pt-1">
            <div className="flex h-7 w-7 items-center justify-center rounded-lg bg-primary text-primary-foreground">
              <Sparkles className="h-4 w-4" />
            </div>
            <span className="text-sm font-semibold">{t("app.name")}</span>
          </div>
          {STEP_KEYS.map((key, i) => {
            const StepIcon = STEP_ICONS[i];
            const done = i < step;
            return (
              <button
                key={key}
                onClick={() => setStep(i)}
                className={cn(
                  "flex h-9 items-center gap-2 rounded-md px-2.5 text-xs transition-colors",
                  i === step
                    ? "bg-primary/10 font-medium text-primary"
                    : "text-muted-foreground hover:bg-accent hover:text-foreground",
                )}
              >
                <span
                  className={cn(
                    "flex h-5 w-5 items-center justify-center rounded-full border text-[10px]",
                    i === step && "border-primary text-primary",
                    done && "border-success bg-success text-primary-foreground",
                  )}
                >
                  {done ? <Check className="h-3 w-3" /> : i + 1}
                </span>
                <StepIcon className="h-3.5 w-3.5" />
                {t(key)}
              </button>
            );
          })}
          <div className="mt-auto px-2 text-[11px] text-muted-foreground">
            {t("wizard.stepProgress", { n: step + 1, total })}
          </div>
        </div>

        {/* 右侧内容 */}
        <div className="flex min-h-[380px] flex-1 flex-col p-6">
          <div>
            <h2 className="flex items-center gap-2 text-base font-semibold">
              <Icon className="h-4 w-4 text-primary" />
              {step === 0 ? t("wizard.title") : t(STEP_KEYS[step])}
            </h2>
            <p className="mt-1 text-xs text-muted-foreground">
              {step === 0 ? t("wizard.subtitle") : t("wizard.stepProgress", { n: step + 1, total })}
            </p>
          </div>

          <div className="mt-5 flex-1">
            {step === 0 && (
              <div className="space-y-3">
                <p className="text-xs text-muted-foreground">{t("wizard.binaryHint")}</p>
                <div className="mono truncate rounded-md border border-border bg-muted/40 px-2.5 py-2 text-xs">
                  {binary?.path ?? t("params.binaryNotSet")}
                </div>
                {binary && (
                  <div className="flex items-center gap-2">
                    <Badge tone="success">{binary.backend}</Badge>
                    <span className="text-[11px] text-muted-foreground">{binary.version}</span>
                  </div>
                )}
                {detectFailed && <p className="text-xs text-warning">{t("wizard.notDetected")}</p>}
                <div className="flex flex-wrap gap-2">
                  <Button size="sm" onClick={() => void autoDetect()} disabled={detecting}>
                    {detecting ? (
                      <Loader2 className="h-3.5 w-3.5 animate-spin" />
                    ) : (
                      <Sparkles className="h-3.5 w-3.5" />
                    )}
                    {t("wizard.autoDetect")}
                  </Button>
                  <Button variant="outline" size="sm" onClick={() => void choose(false)}>
                    <FolderOpen className="h-3.5 w-3.5" />
                    {t("wizard.selectFile")}
                  </Button>
                  <Button variant="outline" size="sm" onClick={() => void choose(true)}>
                    {t("wizard.selectDir")}
                  </Button>
                </div>
              </div>
            )}

            {step === 1 && (
              <div className="space-y-3">
                <p className="text-xs text-muted-foreground">{t("wizard.modelHint")}</p>
                <Label htmlFor="wiz-model">{t("params.model")}</Label>
                <div className="flex gap-2">
                  <Input
                    id="wiz-model"
                    className="mono"
                    value={editingProfile?.model_path ?? ""}
                    onChange={(e) =>
                      editingProfile &&
                      updateProfile(editingProfile.name, { model_path: e.target.value })
                    }
                    placeholder="C:\\models\\model.gguf"
                  />
                  <Button variant="outline" onClick={() => void chooseModel()}>
                    <FolderOpen className="h-3.5 w-3.5" />
                  </Button>
                </div>
              </div>
            )}

            {step === 2 && (
              <div className="space-y-3">
                <p className="text-xs text-muted-foreground">{t("wizard.portHint")}</p>
                <Label htmlFor="wiz-port">{t("params.port")}</Label>
                <div className="flex gap-2">
                  <Input
                    id="wiz-port"
                    type="number"
                    min={1024}
                    max={65535}
                    value={editingProfile?.port ?? 8080}
                    onChange={(e) =>
                      editingProfile &&
                      updateProfile(editingProfile.name, { port: Number(e.target.value) || 0 })
                    }
                  />
                  <Button variant="outline" onClick={() => void checkPort()}>
                    {t("action.detectPort")}
                  </Button>
                </div>
                {portMsg && (
                  <p className={cn("text-xs", portMsg.ok ? "text-success" : "text-warning")}>
                    {portMsg.text}
                  </p>
                )}
                {!portMsg?.ok && freePort && (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => {
                      if (editingProfile) updateProfile(editingProfile.name, { port: freePort });
                      setPortMsg({ ok: true, text: `${freePort}` });
                    }}
                  >
                    {t("action.continue")} :{freePort}
                  </Button>
                )}
              </div>
            )}

            {step === 3 && (
              <div className="space-y-4">
                <div className="space-y-1.5">
                  <Label htmlFor="wiz-lang">{t("settings.language")}</Label>
                  <Select
                    id="wiz-lang"
                    value={lang}
                    onChange={(e) => {
                      const l = e.target.value as Lang;
                      setLang(l);
                      updateGlobal({ language: l });
                    }}
                  >
                    <option value="zh-CN">简体中文</option>
                    <option value="en-US">English</option>
                  </Select>
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="wiz-theme">{t("settings.theme")}</Label>
                  <Select
                    id="wiz-theme"
                    value={theme}
                    onChange={(e) => {
                      const v = e.target.value as Theme;
                      setTheme(v);
                      updateGlobal({ theme: v });
                    }}
                  >
                    <option value="light">{t("settings.theme.light")}</option>
                    <option value="dark">{t("settings.theme.dark")}</option>
                    <option value="system">{t("settings.theme.system")}</option>
                  </Select>
                </div>
              </div>
            )}

            {step === 4 && (
              <div className="space-y-4">
                <p className="text-xs text-muted-foreground">{t("wizard.monitorHint")}</p>
                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-1.5">
                    <Label htmlFor="wiz-window">{t("settings.monitorWindow")}</Label>
                    <Input
                      id="wiz-window"
                      type="number"
                      min={1}
                      max={60}
                      value={cfg.global.monitor_window_minutes}
                      onChange={(e) =>
                        updateGlobal({ monitor_window_minutes: Number(e.target.value) || 5 })
                      }
                    />
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor="wiz-sample">{t("settings.monitorSample")}</Label>
                    <Input
                      id="wiz-sample"
                      type="number"
                      min={0.5}
                      max={10}
                      step={0.5}
                      value={cfg.global.monitor_sample_seconds}
                      onChange={(e) =>
                        updateGlobal({ monitor_sample_seconds: Number(e.target.value) || 1 })
                      }
                    />
                  </div>
                </div>
              </div>
            )}
          </div>

          <div className="mt-6 flex items-center justify-between">
            <Button variant="ghost" size="sm" onClick={() => void finish(true)}>
              {t("wizard.skip")}
            </Button>
            <div className="flex gap-2">
              {step > 0 && (
                <Button variant="outline" size="sm" onClick={() => setStep((s) => s - 1)}>
                  <ChevronLeft className="h-3.5 w-3.5" />
                  {t("wizard.back")}
                </Button>
              )}
              {step < total - 1 ? (
                <Button size="sm" onClick={() => setStep((s) => s + 1)}>
                  {t("wizard.next")}
                  <ChevronRight className="h-3.5 w-3.5" />
                </Button>
              ) : (
                <Button size="sm" onClick={() => void finish(false)}>
                  <Check className="h-3.5 w-3.5" />
                  {t("wizard.finish")}
                </Button>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
