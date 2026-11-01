import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { enable, disable, isEnabled } from "@tauri-apps/plugin-autostart";
import {
  Copy,
  FolderOpen,
  Info,
  KeyRound,
  RefreshCw,
  Save,
  Sparkles,
  Wand2,
} from "lucide-react";
import { api } from "../lib/api";
import { useI18n, type Lang } from "../lib/i18n";
import { useStore } from "../lib/store";
import { useTheme, type Theme } from "../lib/theme";
import { useToast } from "../lib/toast";
import type { ApiStatus } from "../lib/types";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../components/ui/card";
import { Input } from "../components/ui/input";
import { Label } from "../components/ui/label";
import { Select } from "../components/ui/select";
import { Switch } from "../components/ui/switch";

function generateToken(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(24));
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

export function Settings() {
  const { t, lang, setLang } = useI18n();
  const { cfg, updateGlobal, saveCfg, binary, setBinary, setWizardOpen } = useStore();
  const { theme, setTheme } = useTheme();
  const { toast } = useToast();
  const [autoStartActual, setAutoStartActual] = useState<boolean | null>(null);
  const [saving, setSaving] = useState(false);
  const [apiStatus, setApiStatus] = useState<ApiStatus | null>(null);
  const [gpus, setGpus] = useState<
    { index: number; name: string; mem_total_bytes: number | null }[]
  >([]);

  useEffect(() => {
    isEnabled()
      .then(setAutoStartActual)
      .catch(() => setAutoStartActual(null));
    api
      .apiStatus()
      .then(setApiStatus)
      .catch(() => setApiStatus(null));
    api
      .listGpus()
      .then(setGpus)
      .catch(() => setGpus([]));
  }, []);

  if (!cfg) return <div className="p-6 text-sm text-muted-foreground">{t("misc.loading")}</div>;

  const g = cfg.global;

  const save = async () => {
    if (g.monitor_window_minutes < 1 || g.monitor_window_minutes > 60) {
      toast(t("settings.invalidRange"), "error");
      return;
    }
    if (g.monitor_sample_seconds < 0.5 || g.monitor_sample_seconds > 10) {
      toast(t("settings.invalidRange"), "error");
      return;
    }
    let next = { ...cfg, global: { ...g } };
    if (next.global.api_enabled) {
      if (next.global.api_port < 1024) {
        toast(t("params.portRange"), "error");
        return;
      }
      const conflict = next.profiles.find((p) => p.port === next.global.api_port);
      if (conflict) {
        toast(t("settings.portConflict", { port: next.global.api_port }), "error");
        return;
      }
      if (!next.global.api_token) {
        next = {
          ...next,
          global: { ...next.global, api_token: generateToken() },
        };
      }
    }
    setSaving(true);
    try {
      await saveCfg(next, { silent: true });
      api
        .apiStatus()
        .then(setApiStatus)
        .catch(() => setApiStatus(null));
      try {
        if (next.global.autostart) await enable();
        else await disable();
        setAutoStartActual(next.global.autostart);
      } catch (e) {
        toast(t("settings.autostartFailed", { err: String(e) }), "warning");
      }
      toast(t("settings.savedOk"), "success");
    } catch {
      /* toast already shown */
    } finally {
      setSaving(false);
    }
  };

  const chooseBinary = async (directory: boolean) => {
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
      toast(`${info.backend} · ${info.version}`, "success");
    } catch (e) {
      toast(String(e), "error");
    }
  };

  const rescan = async (force: boolean) => {
    try {
      const info = await api.rescanHelp(force);
      setBinary(info);
      toast(`${info.help_args.length} params`, "success");
    } catch (e) {
      toast(String(e), "error");
    }
  };

  const resetToken = () => {
    updateGlobal({ api_token: generateToken() });
    toast(t("settings.tokenGenerated"), "success");
  };

  return (
    <div className="animate-fade-in p-6">
      <div className="mb-5 flex items-start justify-between gap-4">
        <div>
          <h1 className="text-lg font-semibold">{t("settings.title")}</h1>
          <p className="mt-0.5 text-xs text-muted-foreground">{t("settings.subtitle")}</p>
        </div>
        <Button size="sm" onClick={() => void save()} disabled={saving}>
          <Save className="h-3.5 w-3.5" />
          {t("action.save")}
        </Button>
      </div>

      <div className="grid gap-4 xl:grid-cols-2">
        {/* 通用 */}
        <Card>
          <CardHeader>
            <CardTitle>{t("settings.general")}</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="lang">{t("settings.language")}</Label>
              <Select
                id="lang"
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
              <Label htmlFor="theme">{t("settings.theme")}</Label>
              <Select
                id="theme"
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

            <div className="space-y-3 rounded-md border border-border p-3">
              <div className="flex items-center justify-between gap-4">
                <Label htmlFor="tray" className="cursor-pointer">
                  {t("settings.closeToTray")}
                </Label>
                <Switch
                  id="tray"
                  checked={g.close_to_tray}
                  onCheckedChange={(v) => updateGlobal({ close_to_tray: v })}
                />
              </div>
              <div className="flex items-center justify-between gap-4">
                <Label htmlFor="keep" className="cursor-pointer">
                  {t("settings.exitKeepServer")}
                </Label>
                <Switch
                  id="keep"
                  checked={g.exit_keep_server}
                  onCheckedChange={(v) => updateGlobal({ exit_keep_server: v })}
                />
              </div>
              <div className="flex items-center justify-between gap-4">
                <Label htmlFor="autostart" className="cursor-pointer">
                  {t("settings.autostart")}
                </Label>
                <Switch
                  id="autostart"
                  checked={g.autostart}
                  onCheckedChange={(v) => updateGlobal({ autostart: v })}
                />
              </div>
              {autoStartActual !== null && autoStartActual !== g.autostart && (
                <p className="text-[11px] text-warning">
                  {t("misc.settings")}: {autoStartActual ? t("misc.on") : t("misc.off")}
                </p>
              )}
            </div>

            <div className="flex flex-wrap gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setWizardOpen(true)}
              >
                <Wand2 className="h-3.5 w-3.5" />
                {t("settings.wizard")}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  Object.keys(localStorage)
                    .filter((k) => k.startsWith("perch-rec-"))
                    .forEach((k) => localStorage.removeItem(k));
                  toast(t("settings.tipsResetDone"), "success");
                }}
              >
                <Sparkles className="h-3.5 w-3.5" />
                {t("settings.tipsReset")}
              </Button>
            </div>

            <p className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
              <Info className="h-3 w-3" />
              {t("settings.adminHint")}
            </p>
          </CardContent>
        </Card>

        {/* 监控 */}
        <Card className="self-start">
          <CardHeader>
            <CardTitle>{t("settings.monitor")}</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-1.5">
              <Label htmlFor="window">{t("settings.monitorWindow")}</Label>
              <Input
                id="window"
                type="number"
                min={1}
                max={60}
                value={g.monitor_window_minutes}
                onChange={(e) =>
                  updateGlobal({ monitor_window_minutes: Number(e.target.value) || 1 })
                }
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="sample">{t("settings.monitorSample")}</Label>
              <Input
                id="sample"
                type="number"
                min={0.5}
                max={10}
                step={0.5}
                value={g.monitor_sample_seconds}
                onChange={(e) =>
                  updateGlobal({ monitor_sample_seconds: Number(e.target.value) || 1 })
                }
              />
            </div>
            {gpus.length > 0 && (
              <div className="space-y-1.5 sm:col-span-2">
                <Label htmlFor="gpu-index">{t("settings.gpuIndex")}</Label>
                <Select
                  id="gpu-index"
                  value={String(g.gpu_index)}
                  onChange={(e) => updateGlobal({ gpu_index: Number(e.target.value) || 0 })}
                >
                  {gpus.map((gpu) => (
                    <option key={gpu.index} value={gpu.index}>
                      #{gpu.index} {gpu.name}
                      {gpu.mem_total_bytes
                        ? ` · ${(gpu.mem_total_bytes / 1024 / 1024 / 1024).toFixed(1)} GiB`
                        : ""}
                    </option>
                  ))}
                </Select>
              </div>
            )}
            <div className="section-title sm:col-span-2 mt-1">
              {t("settings.thresholds")}
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="alert-cpu">{t("settings.alertCpu")}</Label>
              <Input
                id="alert-cpu"
                type="number"
                min={1}
                max={100}
                value={g.alert_cpu_percent}
                onChange={(e) => updateGlobal({ alert_cpu_percent: Number(e.target.value) || 90 })}
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="alert-mem">{t("settings.alertMem")}</Label>
              <Input
                id="alert-mem"
                type="number"
                min={1}
                max={100}
                value={g.alert_mem_percent}
                onChange={(e) => updateGlobal({ alert_mem_percent: Number(e.target.value) || 90 })}
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="alert-gpu">{t("settings.alertGpu")}</Label>
              <Input
                id="alert-gpu"
                type="number"
                min={1}
                max={100}
                value={g.alert_gpu_util_percent}
                onChange={(e) =>
                  updateGlobal({ alert_gpu_util_percent: Number(e.target.value) || 90 })
                }
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="log-retention">{t("settings.logRetention")}</Label>
              <Input
                id="log-retention"
                type="number"
                min={0}
                max={3650}
                value={g.log_retention_days}
                onChange={(e) =>
                  updateGlobal({ log_retention_days: Math.max(0, Number(e.target.value) || 0) })
                }
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="alert-temp">{t("settings.alertTemp")}</Label>
              <Input
                id="alert-temp"
                type="number"
                min={1}
                max={120}
                value={g.alert_gpu_temp_c}
                onChange={(e) => updateGlobal({ alert_gpu_temp_c: Number(e.target.value) || 85 })}
              />
            </div>
          </CardContent>
        </Card>

        {/* 二进制 */}
        <Card>
          <CardHeader>
            <CardTitle>{t("settings.binary")}</CardTitle>
            <CardDescription>{t("params.binaryHint")}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="mono truncate rounded-md border border-border bg-muted/40 px-2.5 py-1.5 text-xs">
              {binary?.path ?? t("settings.binaryNotSet")}
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <Button variant="outline" size="sm" onClick={() => void chooseBinary(false)}>
                <FolderOpen className="h-3.5 w-3.5" />
                {t("action.browse")}
              </Button>
              <Button variant="outline" size="sm" onClick={() => void chooseBinary(true)}>
                {t("params.binary")}
              </Button>
              <Button variant="ghost" size="sm" onClick={() => void rescan(false)}>
                <RefreshCw className="h-3.5 w-3.5" />
                {t("action.rescan")}
              </Button>
              <Button variant="ghost" size="sm" onClick={() => void rescan(true)}>
                {t("action.forceRescan")}
              </Button>
            </div>
            {binary && (
              <div className="flex items-center gap-2">
                <Badge tone="success">{binary.backend}</Badge>
                <span className="text-[11px] text-muted-foreground">
                  {binary.version} · {binary.help_args.length} params
                </span>
              </div>
            )}
          </CardContent>
        </Card>

        {/* 本地 API */}
        <Card>
          <CardHeader>
            <CardTitle>{t("settings.api")}</CardTitle>
            <CardDescription>{t("settings.apiHint")}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center justify-between gap-4">
              <Label htmlFor="api-enable" className="cursor-pointer">
                {t("settings.apiEnable")}
              </Label>
              <Switch
                id="api-enable"
                checked={g.api_enabled}
                onCheckedChange={(v) =>
                  updateGlobal({
                    api_enabled: v,
                    api_token: v && !g.api_token ? generateToken() : g.api_token,
                  })
                }
              />
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="api-port">{t("settings.apiPort")}</Label>
              <Input
                id="api-port"
                type="number"
                min={1024}
                max={65535}
                disabled={!g.api_enabled}
                value={g.api_port}
                onChange={(e) => updateGlobal({ api_port: Number(e.target.value) || 3999 })}
              />
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="api-token">{t("settings.apiToken")}</Label>
              <div className="flex gap-2">
                <Input
                  id="api-token"
                  readOnly
                  disabled={!g.api_enabled}
                  value={g.api_token}
                  className="mono text-xs"
                />
                <Button
                  variant="outline"
                  size="icon"
                  disabled={!g.api_enabled || !g.api_token}
                  title={t("settings.copyToken")}
                  onClick={() => void navigator.clipboard.writeText(g.api_token)}
                >
                  <Copy className="h-3.5 w-3.5" />
                </Button>
                <Button
                  variant="outline"
                  size="icon"
                  disabled={!g.api_enabled}
                  title={t("settings.apiReset")}
                  onClick={resetToken}
                >
                  <KeyRound className="h-3.5 w-3.5" />
                </Button>
              </div>
            </div>

            {g.api_enabled && (
              <div className="space-y-1 rounded-md border border-border bg-muted/30 p-2.5 text-[11px]">
                <div className="flex items-center gap-2">
                  <Badge tone={apiStatus?.running ? "success" : "muted"}>
                    {apiStatus?.running ? t("settings.apiRunning") : t("settings.apiStopped")}
                  </Badge>
                  {apiStatus?.running && (
                    <span className="mono text-muted-foreground">
                      127.0.0.1:{apiStatus.port}
                    </span>
                  )}
                </div>
                {apiStatus?.error && <p className="text-destructive">{apiStatus.error}</p>}
                <p className="text-muted-foreground">{t("settings.apiEndpoints")}</p>
                {g.api_token && (
                  <p className="text-muted-foreground">{t("settings.apiAuthHint")}</p>
                )}
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
