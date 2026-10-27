import { useEffect, useMemo, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  AlertTriangle,
  Check,
  ChevronDown,
  Copy,
  FileDown,
  FolderOpen,
  RefreshCw,
  Save,
  Search,
  Star,
  StarOff,
  Terminal,
} from "lucide-react";
import { api, type PortStatus } from "../lib/api";
import { useI18n } from "../lib/i18n";
import { useStore } from "../lib/store";
import { useToast } from "../lib/toast";
import type { CommandPreview, HelpArg, Profile } from "../lib/types";
import { cn } from "../lib/utils";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../components/ui/card";
import { Dialog } from "../components/ui/dialog";
import { Input } from "../components/ui/input";
import { Label } from "../components/ui/label";
import { Select } from "../components/ui/select";
import { Switch } from "../components/ui/switch";
import { Textarea } from "../components/ui/textarea";

const FAV_KEY = "perch-fav-params";
const COMMON_GROUPS = new Set([
  "model",
  "m",
  "host",
  "port",
  "p",
  "ngl",
  "gpu-layers",
  "n-gpu-layers",
  "threads",
  "t",
  "ctx-size",
  "c",
  "flash-attn",
  "fa",
]);

function flagOf(arg: HelpArg): string {
  return arg.short && arg.name === arg.short ? `-${arg.short}` : `--${arg.name}`;
}

function loadFavorites(): string[] {
  try {
    const raw = localStorage.getItem(FAV_KEY);
    if (raw) return JSON.parse(raw) as string[];
  } catch {
    /* ignore */
  }
  return [];
}

export function Parameters() {
  const { t } = useI18n();
  const { cfg, editingProfile, updateProfile, saveCfg, binary, setBinary, refreshBinary } =
    useStore();
  const { toast } = useToast();

  const [dirty, setDirty] = useState(false);
  const [preview, setPreview] = useState<CommandPreview | null>(null);
  const [search, setSearch] = useState("");
  const [tab, setTab] = useState<"all" | "favorites">("all");
  const [favorites, setFavorites] = useState<string[]>(loadFavorites);
  const [advancedOpen, setAdvancedOpen] = useState(true);
  const [absolute, setAbsolute] = useState(true);
  const [resolving, setResolving] = useState(false);
  const [previewTick, setPreviewTick] = useState(0);
  const [portState, setPortState] = useState<PortStatus | null>(null);
  const [freePort, setFreePort] = useState<number | null>(null);
  const [pendingHost, setPendingHost] = useState<string | null>(null);

  const profile = editingProfile;

  const update = (patch: Partial<Profile>) => {
    if (!profile) return;
    updateProfile(profile.name, patch);
    setDirty(true);
  };

  useEffect(() => {
    if (!profile) return;
    let cancelled = false;
    const timer = window.setTimeout(() => {
      api
        .buildCommand(profile)
        .then((p) => {
          if (!cancelled) setPreview(p);
        })
        .catch((e) => {
          if (!cancelled) toast(String(e), "error");
        });
    }, 150);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [profile, previewTick, toast]);

  const saveAll = async () => {
    if (!cfg) return;
    try {
      await saveCfg(cfg, { silent: true });
      setDirty(false);
      toast(t("action.saved"), "success");
    } catch {
      /* toast already shown */
    }
  };

  const resolvePath = async (path: string) => {
    setResolving(true);
    try {
      const info = await api.resolveBinary(path);
      setBinary(info);
      const current = cfg;
      if (current) {
        await saveCfg(
          { ...current, global: { ...current.global, binary_path: info.path } },
          { silent: true },
        );
      }
      setPreviewTick((v) => v + 1);
      toast(`${info.backend} · ${info.version}`, "success");
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setResolving(false);
    }
  };

  const chooseFile = async () => {
    const picked = await open({
      multiple: false,
      directory: false,
      title: t("params.selectBinary"),
      filters: [{ name: "llama-server", extensions: ["exe"] }],
    });
    if (typeof picked === "string") await resolvePath(picked);
  };

  const chooseDir = async () => {
    const picked = await open({
      multiple: false,
      directory: true,
      title: t("params.selectBinary"),
    });
    if (typeof picked === "string") await resolvePath(picked);
  };

  const chooseModel = async () => {
    const picked = await open({
      multiple: false,
      directory: false,
      title: t("params.selectModel"),
      filters: [{ name: "GGUF", extensions: ["gguf"] }],
    });
    if (typeof picked === "string") update({ model_path: picked });
  };

  const rescan = async (force: boolean) => {
    try {
      const info = await api.rescanHelp(force);
      setBinary(info);
      setPreviewTick((v) => v + 1);
      toast(`${info.help_args.length} params`, "success");
    } catch (e) {
      toast(String(e), "error");
    }
  };

  const checkPort = async () => {
    if (!profile) return;
    try {
      const st = await api.checkPort(profile.port);
      if (st.available) {
        toast(`${t("params.port")} ${st.port} ✓`, "success");
        return;
      }
      setPortState(st);
      try {
        setFreePort(await api.findFreePort(Math.max(profile.port + 1, 1024)));
      } catch {
        setFreePort(null);
      }
    } catch (e) {
      toast(String(e), "error");
    }
  };

  const changeHost = (value: string) => {
    if (!profile) return;
    if (value === "0.0.0.0") {
      setPendingHost(value);
      return;
    }
    update({ host: value });
  };

  const copyPreview = async () => {
    if (!preview) return;
    try {
      await navigator.clipboard.writeText(preview.preview);
      toast(t("action.copied"), "success");
    } catch {
      toast(t("error.copyFailed"), "error");
    }
  };

  const exportBat = async () => {
    if (!profile) return;
    const path = await save({
      title: t("action.exportBat"),
      defaultPath: `${profile.name}.bat`,
      filters: [{ name: "Batch", extensions: ["bat"] }],
    });
    if (!path) return;
    try {
      await api.exportBat(profile, path, absolute);
      toast(t("action.saved"), "success");
    } catch (e) {
      toast(String(e), "error");
    }
  };

  const toggleFavorite = (name: string) => {
    setFavorites((prev) => {
      const next = prev.includes(name) ? prev.filter((n) => n !== name) : [...prev, name];
      localStorage.setItem(FAV_KEY, JSON.stringify(next));
      return next;
    });
  };

  const addParam = (arg: HelpArg) => {
    if (!profile) return;
    const token = arg.takes_value ? `${flagOf(arg)} ${arg.value || "VALUE"}` : flagOf(arg);
    const text = profile.extra_params.trim();
    update({ extra_params: text ? `${text} ${token}` : token });
  };

  const helpArgs = useMemo(() => {
    const all = (binary?.help_args ?? []).filter((a) => !COMMON_GROUPS.has(a.name));
    const q = search.trim().toLowerCase();
    const filtered = q
      ? all.filter(
          (a) =>
            a.name.toLowerCase().includes(q) ||
            a.short.toLowerCase().includes(q) ||
            a.description.toLowerCase().includes(q),
        )
      : all;
    const list = tab === "favorites" ? filtered.filter((a) => favorites.includes(a.name)) : filtered;
    return [...list].sort((a, b) => {
      const fa = favorites.includes(a.name) ? 0 : 1;
      const fb = favorites.includes(b.name) ? 0 : 1;
      return fa - fb;
    });
  }, [binary, search, tab, favorites]);

  const warnings = preview?.warnings ?? [];

  if (!profile) {
    return (
      <div className="p-6 text-sm text-muted-foreground">{t("misc.loading")}</div>
    );
  }

  const thresholdHint = profile.port < 1024 || profile.port > 65535;

  return (
    <div className="animate-fade-in p-6">
      <div className="mb-5 flex items-start justify-between gap-4">
        <div>
          <h1 className="text-lg font-semibold">{t("params.title")}</h1>
          <p className="mt-0.5 text-xs text-muted-foreground">{t("params.subtitle")}</p>
        </div>
        <div className="flex items-center gap-2">
          {dirty && <Badge tone="warning">{t("params.unsaved")}</Badge>}
          <Button variant="outline" size="sm" onClick={() => void exportBat()}>
            <FileDown className="h-3.5 w-3.5" />
            {t("action.exportBat")}
          </Button>
          <Button size="sm" onClick={() => void saveAll()}>
            <Save className="h-3.5 w-3.5" />
            {t("action.save")}
          </Button>
        </div>
      </div>

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_400px]">
        <div className="space-y-4">
          {/* 二进制 */}
          <Card>
            <CardHeader className="flex-row items-center justify-between">
              <div>
                <CardTitle>{t("params.binary")}</CardTitle>
                <CardDescription>{t("params.binaryHint")}</CardDescription>
              </div>
              {binary && <Badge tone="success">{binary.backend}</Badge>}
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="flex items-center gap-2">
                <div className="mono min-w-0 flex-1 truncate rounded-md border border-border bg-muted/40 px-2.5 py-1.5">
                  {binary?.path ?? t("params.binaryNotSet")}
                </div>
                <Button variant="outline" size="sm" onClick={() => void chooseFile()} disabled={resolving}>
                  <FolderOpen className="h-3.5 w-3.5" />
                  {t("action.browse")}
                </Button>
                <Button variant="outline" size="sm" onClick={() => void chooseDir()} disabled={resolving}>
                  {t("params.binary")}
                </Button>
              </div>
              <div className="flex flex-wrap items-center gap-2 text-[11px] text-muted-foreground">
                {binary && (
                  <span>
                    {t("params.version")}: {binary.version} · {binary.help_args.length} params
                  </span>
                )}
                <div className="ml-auto flex gap-2">
                  <Button variant="ghost" size="sm" onClick={() => void refreshBinary()}>
                    <RefreshCw className="h-3.5 w-3.5" />
                    {t("action.rescan")}
                  </Button>
                  <Button variant="ghost" size="sm" onClick={() => void rescan(true)}>
                    {t("action.forceRescan")}
                  </Button>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* 常用参数 */}
          <Card>
            <CardHeader>
              <CardTitle>{t("params.common")}</CardTitle>
              <CardDescription>{t("params.savedHint")}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-1.5">
                <Label htmlFor="model">{t("params.model")}</Label>
                <div className="flex gap-2">
                  <Input
                    id="model"
                    value={profile.model_path}
                    onChange={(e) => update({ model_path: e.target.value })}
                    placeholder="C:\\models\\model.gguf"
                    className="mono"
                  />
                  <Button variant="outline" size="md" onClick={() => void chooseModel()}>
                    <FolderOpen className="h-3.5 w-3.5" />
                  </Button>
                </div>
              </div>

              <div className="grid gap-4 sm:grid-cols-2">
                <div className="space-y-1.5">
                  <Label htmlFor="host">{t("params.host")}</Label>
                  <Select
                    id="host"
                    value={profile.host}
                    onChange={(e) => changeHost(e.target.value)}
                  >
                    <option value="127.0.0.1">127.0.0.1</option>
                    <option value="0.0.0.0">0.0.0.0</option>
                    <option value="::1">::1</option>
                  </Select>
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="port">{t("params.port")}</Label>
                  <div className="flex gap-2">
                    <Input
                      id="port"
                      type="number"
                      min={1}
                      max={65535}
                      value={profile.port}
                      onChange={(e) => update({ port: Number(e.target.value) || 0 })}
                    />
                    <Button variant="outline" size="md" onClick={() => void checkPort()}>
                      {t("action.detectPort")}
                    </Button>
                  </div>
                </div>
              </div>

              <div className="grid gap-4 sm:grid-cols-3">
                <div className="space-y-1.5">
                  <Label htmlFor="ngl">{t("params.ngl")}</Label>
                  <Input
                    id="ngl"
                    type="number"
                    value={profile.ngl >= 0 ? profile.ngl : ""}
                    placeholder={t("params.nglAuto")}
                    onChange={(e) =>
                      update({ ngl: e.target.value === "" ? -1 : Number(e.target.value) })
                    }
                  />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="threads">{t("params.threads")}</Label>
                  <Input
                    id="threads"
                    type="number"
                    value={profile.threads > 0 ? profile.threads : ""}
                    placeholder={t("params.threadsAuto")}
                    onChange={(e) => update({ threads: Number(e.target.value) || 0 })}
                  />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="ctx">{t("params.ctxSize")}</Label>
                  <Input
                    id="ctx"
                    type="number"
                    value={profile.ctx_size > 0 ? profile.ctx_size : ""}
                    placeholder={t("params.ctxAuto")}
                    onChange={(e) => update({ ctx_size: Number(e.target.value) || 0 })}
                  />
                </div>
              </div>

              <div className="flex items-center justify-between rounded-md border border-border px-3 py-2">
                <Label htmlFor="fa" className="cursor-pointer">
                  {t("params.flashAttn")}
                </Label>
                <Switch
                  id="fa"
                  checked={profile.flash_attn}
                  onCheckedChange={(v) => update({ flash_attn: v })}
                />
              </div>

              {thresholdHint && (
                <p className="flex items-center gap-1.5 text-xs text-warning">
                  <AlertTriangle className="h-3.5 w-3.5" />
                  {t("params.portRange")}
                </p>
              )}
            </CardContent>
          </Card>

          {/* 高级参数 */}
          <Card>
            <CardHeader className="flex-row items-center justify-between">
              <div>
                <CardTitle>{t("params.advanced")}</CardTitle>
                <CardDescription>{t("params.advancedHint")}</CardDescription>
              </div>
              <Button variant="ghost" size="icon" onClick={() => setAdvancedOpen((v) => !v)}>
                <ChevronDown
                  className={cn("h-4 w-4 transition-transform", advancedOpen && "rotate-180")}
                />
              </Button>
            </CardHeader>
            {advancedOpen && (
              <CardContent className="space-y-3">
                <div className="flex items-center gap-2">
                  <div className="relative flex-1">
                    <Search className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
                    <Input
                      value={search}
                      onChange={(e) => setSearch(e.target.value)}
                      placeholder={t("params.searchParams")}
                      className="pl-8"
                    />
                  </div>
                  <div className="flex rounded-md border border-border p-0.5">
                    {(["all", "favorites"] as const).map((k) => (
                      <button
                        key={k}
                        onClick={() => setTab(k)}
                        className={cn(
                          "rounded px-2.5 py-1 text-xs transition-colors",
                          tab === k
                            ? "bg-primary/10 text-primary"
                            : "text-muted-foreground hover:text-foreground",
                        )}
                      >
                        {k === "all" ? t("params.advanced") : t("params.favorites")}
                      </button>
                    ))}
                  </div>
                </div>

                <div className="max-h-72 space-y-1 overflow-y-auto pr-1">
                  {helpArgs.length === 0 && (
                    <div className="py-6 text-center text-xs text-muted-foreground">
                      {tab === "favorites" ? t("params.noFavorite") : t("params.binaryNotSet")}
                    </div>
                  )}
                  {helpArgs.map((arg) => {
                    const fav = favorites.includes(arg.name);
                    return (
                      <div
                        key={`${arg.name}-${arg.short}`}
                        className="group flex items-start gap-2 rounded-md px-2 py-1.5 hover:bg-accent/60"
                      >
                        <button
                          onClick={() => toggleFavorite(arg.name)}
                          className={cn(
                            "mt-0.5 shrink-0 text-muted-foreground transition-colors hover:text-warning",
                            fav && "text-warning",
                          )}
                        >
                          {fav ? <Star className="h-3.5 w-3.5 fill-current" /> : <StarOff className="h-3.5 w-3.5" />}
                        </button>
                        <button
                          onClick={() => addParam(arg)}
                          className="min-w-0 flex-1 text-left"
                          title={t("params.copyTpl")}
                        >
                          <div className="flex items-center gap-2">
                            <span className="mono text-xs font-medium text-primary">
                              {flagOf(arg)}
                              {arg.takes_value && (
                                <span className="text-muted-foreground"> {arg.value}</span>
                              )}
                            </span>
                            {arg.short && arg.name !== arg.short && (
                              <span className="mono text-[10px] text-muted-foreground">-{arg.short}</span>
                            )}
                          </div>
                          {arg.description && (
                            <div className="mt-0.5 truncate text-[11px] text-muted-foreground">
                              {arg.description}
                            </div>
                          )}
                        </button>
                        <button
                          onClick={() => addParam(arg)}
                          className="mt-0.5 shrink-0 rounded p-1 text-muted-foreground opacity-0 transition-opacity hover:bg-accent hover:text-foreground group-hover:opacity-100"
                          title={t("params.add")}
                        >
                          <Check className="h-3.5 w-3.5" />
                        </button>
                      </div>
                    );
                  })}
                </div>
              </CardContent>
            )}
          </Card>

          {/* 自定义参数 */}
          <Card>
            <CardHeader>
              <CardTitle>{t("params.custom")}</CardTitle>
              <CardDescription>{t("params.customHint")}</CardDescription>
            </CardHeader>
            <CardContent>
              <Textarea
                value={profile.extra_params}
                onChange={(e) => update({ extra_params: e.target.value })}
                className="mono min-h-24"
                placeholder="--temp 0.7 --top-k 40 --repeat-penalty 1.1"
                spellCheck={false}
              />
            </CardContent>
          </Card>

          {/* 工作目录与环境变量 */}
          <Card>
            <CardHeader>
              <CardTitle>{t("params.workdir")}</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex gap-2">
                <Input
                  value={profile.work_dir}
                  onChange={(e) => update({ work_dir: e.target.value })}
                  className="mono"
                  placeholder="C:\\llama.cpp"
                />
                <Button
                  variant="outline"
                  onClick={async () => {
                    const picked = await open({ directory: true, multiple: false });
                    if (typeof picked === "string") update({ work_dir: picked });
                  }}
                >
                  <FolderOpen className="h-3.5 w-3.5" />
                </Button>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="env">{t("params.env")}</Label>
                <Textarea
                  id="env"
                  value={profile.env_vars.join("\n")}
                  onChange={(e) => update({ env_vars: e.target.value.split(/\r?\n/) })}
                  placeholder={"CUDA_VISIBLE_DEVICES=0"}
                  className="mono min-h-20"
                  spellCheck={false}
                />
                <p className="text-[11px] text-muted-foreground">{t("params.envHint")}</p>
              </div>
            </CardContent>
          </Card>
        </div>

        {/* 预览侧栏 */}
        <div className="space-y-4 xl:sticky xl:top-4 xl:self-start">
          <Card>
            <CardHeader className="flex-row items-center justify-between">
              <CardTitle className="flex items-center gap-1.5">
                <Terminal className="h-3.5 w-3.5" />
                {t("params.preview")}
              </CardTitle>
              <div className="flex gap-1">
                <Button variant="ghost" size="icon" onClick={() => void copyPreview()} title={t("action.copy")}>
                  <Copy className="h-3.5 w-3.5" />
                </Button>
              </div>
            </CardHeader>
            <CardContent className="space-y-3">
              <pre className="mono max-h-64 overflow-auto whitespace-pre-wrap break-all rounded-md border border-border bg-muted/40 p-2.5 text-[11px] leading-relaxed">
                {preview?.preview ?? "—"}
              </pre>
              <div className="flex items-center justify-between text-[11px] text-muted-foreground">
                <label className="flex cursor-pointer items-center gap-1.5">
                  <input
                    type="checkbox"
                    checked={absolute}
                    onChange={(e) => setAbsolute(e.target.checked)}
                    className="accent-primary"
                  />
                  {t("params.batHint")}
                </label>
              </div>
            </CardContent>
          </Card>

          {(warnings.length > 0 ||
            (preview?.overridden.length ?? 0) > 0 ||
            (preview?.duplicated.length ?? 0) > 0) && (
            <Card>
              <CardHeader>
                <CardTitle className="text-warning">{t("dashboard.alert")}</CardTitle>
              </CardHeader>
              <CardContent className="space-y-1.5 text-xs">
                {warnings.map((w) => (
                  <p key={w} className="flex items-start gap-1.5 text-warning">
                    <AlertTriangle className="mt-0.5 h-3 w-3 shrink-0" />
                    {w}
                  </p>
                ))}
                {preview?.overridden.map((name) => (
                  <p key={`o-${name}`} className="flex items-start gap-1.5 text-warning">
                    <AlertTriangle className="mt-0.5 h-3 w-3 shrink-0" />
                    {t("params.overrideWarn", { name })}
                  </p>
                ))}
                {preview?.duplicated.map((name) => (
                  <p key={`d-${name}`} className="flex items-start gap-1.5 text-warning">
                    <AlertTriangle className="mt-0.5 h-3 w-3 shrink-0" />
                    {t("params.duplicateWarn", { name })}
                  </p>
                ))}
              </CardContent>
            </Card>
          )}
        </div>
      </div>

      <Dialog
        open={pendingHost !== null}
        onClose={() => setPendingHost(null)}
        title={t("params.insecureConfirm")}
        description={t("params.invalidHost")}
        footer={
          <>
            <Button variant="outline" onClick={() => setPendingHost(null)}>
              {t("action.cancel")}
            </Button>
            <Button
              variant="destructive"
              onClick={() => {
                if (pendingHost) update({ host: pendingHost });
                setPendingHost(null);
              }}
            >
              {t("action.confirm")}
            </Button>
          </>
        }
      />

      <Dialog
        open={portState !== null}
        onClose={() => setPortState(null)}
        title={`${t("error.portInUse")} :${portState?.port ?? ""}`}
        description={
          portState?.process_name
            ? `${portState.process_name} (PID ${portState.pid ?? "?"})`
            : undefined
        }
        footer={
          <>
            <Button variant="outline" onClick={() => setPortState(null)}>
              {t("action.cancel")}
            </Button>
            {freePort && (
              <Button
                onClick={() => {
                  update({ port: freePort });
                  setPortState(null);
                }}
              >
                {t("action.continue")} :{freePort}
              </Button>
            )}
          </>
        }
      />
    </div>
  );
}
