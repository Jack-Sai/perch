import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import {
  Activity,
  AlertTriangle,
  ChevronDown,
  Copy,
  Cpu,
  ExternalLink,
  Gauge,
  HardDrive,
  Play,
  RotateCw,
  Square,
  Terminal,
  Thermometer,
} from "lucide-react";
import { save } from "@tauri-apps/plugin-dialog";
import { api, type PortStatus } from "../lib/api";
import { useI18n } from "../lib/i18n";
import { useStore } from "../lib/store";
import { useToast } from "../lib/toast";
import { statusError, statusKind } from "../lib/types";
import type { CommandPreview, Metrics } from "../lib/types";
import {
  cn,
  formatBytes,
  formatDuration,
  formatPercent,
  formatTime,
  lttb,
} from "../lib/utils";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/card";
import { Dialog } from "../components/ui/dialog";
import { Select } from "../components/ui/select";
import { StatusBadge } from "../components/StatusBadge";

interface Sample {
  t: number;
  cpu: number;
  mem: number;
  gpuUtil: number;
  gpuMem: number;
  gpuTemp: number;
}

function MetricCard({
  icon: Icon,
  label,
  value,
  sub,
  tone,
  children,
}: {
  icon: typeof Cpu;
  label: string;
  value: string;
  sub?: string;
  tone?: "default" | "warning" | "destructive";
  children?: React.ReactNode;
}) {
  return (
    <Card>
      <CardContent className="p-4">
        <div className="flex items-center justify-between text-xs text-muted-foreground">
          <span className="flex items-center gap-1.5">
            <Icon className="h-3.5 w-3.5" />
            {label}
          </span>
          {children}
        </div>
        <div
          className={cn(
            "mt-2 text-xl font-semibold tabular-nums",
            tone === "warning" && "text-warning",
            tone === "destructive" && "text-destructive",
          )}
        >
          {value}
        </div>
        {sub && <div className="mt-0.5 text-[11px] text-muted-foreground">{sub}</div>}
      </CardContent>
    </Card>
  );
}

export function Dashboard() {
  const { t } = useI18n();
  const {
    cfg,
    status,
    pid,
    editing,
    editingProfile,
    runningProfile,
    startedAt,
    binary,
    selectProfile,
    start,
    stop,
    restart,
    openWebui,
  } = useStore();
  const { toast } = useToast();

  const kind = statusKind(status);
  const sampleSeconds = cfg?.global.monitor_sample_seconds ?? 1;
  const windowMinutes = cfg?.global.monitor_window_minutes ?? 5;
  const cpuAlert = cfg?.global.alert_cpu_percent ?? 90;
  const memAlert = cfg?.global.alert_mem_percent ?? 90;
  const gpuAlert = cfg?.global.alert_gpu_util_percent ?? 90;
  const tempAlert = cfg?.global.alert_gpu_temp_c ?? 85;

  const [metrics, setMetrics] = useState<Metrics | null>(null);
  const [samples, setSamples] = useState<Sample[]>([]);
  const [now, setNow] = useState(Date.now());
  const [logLines, setLogLines] = useState<string[]>([]);
  const [logOpen, setLogOpen] = useState(true);
  const [preview, setPreview] = useState<CommandPreview | null>(null);
  const [portInfo, setPortInfo] = useState<PortStatus | null>(null);
  const [freePort, setFreePort] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const logRef = useRef<HTMLPreElement>(null);

  const runningProfileObj = useMemo(
    () => cfg?.profiles.find((p) => p.name === runningProfile) ?? null,
    [cfg, runningProfile],
  );

  // CPU/GPU 指标轮询
  useEffect(() => {
    if (kind !== "running") {
      setMetrics(null);
      return;
    }
    let cancelled = false;
    let timer: number | undefined;
    const tick = async () => {
      try {
        const m = await api.getMetrics();
        if (cancelled) return;
        setMetrics(m);
        setSamples((prev) => {
          const next = [
            ...prev,
            {
              t: Date.now(),
              cpu: m.cpu_percent,
              mem: m.mem_bytes / 1024 / 1024,
              gpuUtil: m.gpu_util_percent ?? 0,
              gpuMem: (m.gpu_mem_used_bytes ?? 0) / 1024 / 1024,
              gpuTemp: m.gpu_temp_c ?? 0,
            },
          ];
          const max = Math.max(60, Math.ceil((windowMinutes * 60) / sampleSeconds) + 10);
          if (next.length > max) next.splice(0, next.length - max);
          return next;
        });
      } catch {
        /* transient */
      } finally {
        if (!cancelled) {
          timer = window.setTimeout(tick, Math.max(500, sampleSeconds * 1000));
        }
      }
    };
    tick();
    return () => {
      cancelled = true;
      if (timer) window.clearTimeout(timer);
    };
  }, [kind, sampleSeconds, windowMinutes]);

  // 运行时长计时
  useEffect(() => {
    if (!startedAt) return;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [startedAt]);

  // 日志轮询
  useEffect(() => {
    if (!["running", "launching", "error"].includes(kind)) {
      setLogLines([]);
      return;
    }
    let cancelled = false;
    const load = async () => {
      try {
        const lines = await api.readServerLog(200);
        if (!cancelled) setLogLines(lines);
      } catch {
        /* ignore */
      }
    };
    void load();
    const timer = window.setInterval(load, 2000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [kind]);

  // 自动滚动日志
  useEffect(() => {
    if (logOpen && logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [logLines, logOpen]);

  // 错误诊断：构建完整命令行
  useEffect(() => {
    if (kind !== "error") {
      setPreview(null);
      return;
    }
    const target = runningProfileObj ?? editingProfile;
    if (!target) return;
    api
      .buildCommand(target)
      .then(setPreview)
      .catch(() => setPreview(null));
  }, [kind, runningProfileObj, editingProfile]);

  const startFlow = useCallback(async () => {
    const profile = editingProfile;
    if (!profile) return;
    if (!binary) {
      toast(t("error.binaryMissing"), "error");
      return;
    }
    if (!profile.model_path.trim()) {
      toast("未指定模型路径", "error");
      return;
    }
    setBusy(true);
    try {
      const st = await api.checkPort(profile.port);
      if (!st.available) {
        setPortInfo(st);
        try {
          setFreePort(await api.findFreePort(Math.max(profile.port + 1, 1024)));
        } catch {
          setFreePort(null);
        }
        return;
      }
      await start(profile);
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setBusy(false);
    }
  }, [binary, editingProfile, start, t, toast]);

  const startWithPort = async (port: number) => {
    if (!editingProfile) return;
    const p = { ...editingProfile, port };
    setPortInfo(null);
    setBusy(true);
    try {
      await start(p);
    } finally {
      setBusy(false);
    }
  };

  const diagnosisKey = useMemo(() => {
    if (kind !== "error") return null;
    const text = `${statusError(status)}\n${logLines.join("\n")}`.toLowerCase();
    if (/启动超时|timed out|timeout/.test(text)) return "error.diag.timeout";
    if (/address already in use|couldn't bind|bind.*fail|10048|端口被占用/.test(text))
      return "error.diag.port";
    if (/out of memory|failed to allocate|cuda error.*memory|显存不足/.test(text))
      return "error.diag.vram";
    if (/failed to load model|error loading model|invalid model|not a valid gguf|failed to open.*gguf/.test(text))
      return "error.diag.model";
    if (/unknown argument|invalid argument|unrecognized option|error: invalid/.test(text))
      return "error.diag.arg";
    if (/access denied|permission denied|privilege|拒绝访问/.test(text))
      return "error.diag.permission";
    if (/no such file|not found|cannot find|binary/.test(text)) return "error.diag.binary";
    if (/backend|no usable gpu|device.*not supported/.test(text)) return "error.diag.backend";
    return "error.diag.unknown";
  }, [kind, status, logLines]);

  const alerts: string[] = [];
  if (metrics && metrics.cpu_percent >= cpuAlert)
    alerts.push(`${t("dashboard.cpu")} ${formatPercent(metrics.cpu_percent)}`);
  if (metrics && metrics.mem_percent >= memAlert)
    alerts.push(`${t("dashboard.memory")} ${formatPercent(metrics.mem_percent)}`);
  if (metrics?.gpu_util_percent != null && metrics.gpu_util_percent >= gpuAlert)
    alerts.push(`${t("dashboard.gpuUtil")} ${formatPercent(metrics.gpu_util_percent)}`);
  if (metrics?.gpu_temp_c != null && metrics.gpu_temp_c >= tempAlert)
    alerts.push(`${t("dashboard.gpuTemp")} ${metrics.gpu_temp_c.toFixed(0)}°C`);

  const exportSamples = useCallback(
    async (format: "csv" | "json") => {
      if (samples.length === 0) return;
      const stamp = new Date().toISOString().slice(0, 19).replace(/[:T]/g, "-");
      try {
        const path = await save({
          defaultPath: `perch-metrics-${stamp}.${format}`,
          filters: [
            {
              name: format === "csv" ? "CSV" : "JSON",
              extensions: [format],
            },
          ],
        });
        if (!path) return;
        let content: string;
        if (format === "csv") {
          const header =
            "timestamp,epoch_ms,cpu_percent,mem_mb,gpu_util_percent,gpu_mem_used_mb,gpu_temp_c";
          const rows = samples.map((s) =>
            [
              new Date(s.t).toISOString(),
              s.t,
              s.cpu.toFixed(2),
              s.mem.toFixed(1),
              s.gpuUtil.toFixed(2),
              s.gpuMem.toFixed(1),
              s.gpuTemp.toFixed(1),
            ].join(","),
          );
          content = [header, ...rows].join("\r\n");
        } else {
          content = JSON.stringify(
            samples.map((s) => ({
              timestamp: new Date(s.t).toISOString(),
              epoch_ms: s.t,
              cpu_percent: s.cpu,
              mem_mb: s.mem,
              gpu_util_percent: s.gpuUtil,
              gpu_mem_used_mb: s.gpuMem,
              gpu_temp_c: s.gpuTemp,
            })),
            null,
            2,
          );
        }
        await api.saveTextFile(path, content);
        toast(t("dashboard.exported"));
      } catch (e) {
        toast(`${t("dashboard.exportFailed")}: ${String(e)}`, "error");
      }
    },
    [samples, t, toast],
  );

  const chartData = useMemo(() => {
    if (samples.length <= 500) {
      return samples.map((s) => ({ ...s, label: formatTime(s.t) }));
    }
    const picked = lttb(
      samples.map((s) => ({ x: s.t, y: s.cpu, ...s })),
      500,
    );
    return picked.map((s) => ({ ...s, label: formatTime(s.t) }));
  }, [samples]);

  const gpuAvailable = !!metrics && metrics.gpu_name !== null;
  const running = kind === "running";
  const launching = kind === "launching";
  const stopping = kind === "stopping";

  return (
    <div className="animate-fade-in space-y-4 p-6">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 className="text-lg font-semibold">{t("dashboard.title")}</h1>
          <p className="mt-0.5 text-xs text-muted-foreground">{t("dashboard.subtitle")}</p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Select
            value={editing}
            onChange={(e) => void selectProfile(e.target.value)}
            className="h-8 w-40 text-xs"
          >
            {cfg?.profiles.map((p) => (
              <option key={p.name} value={p.name}>
                {p.name}
              </option>
            ))}
          </Select>
          <div className="mx-1 h-5 w-px bg-border" />
          <Button variant="outline" size="sm" onClick={() => void openWebui()} disabled={!running}>
            <ExternalLink className="h-3.5 w-3.5" />
            {t("action.openWebui")}
          </Button>
          {running || launching ? (
            <Button
              variant="destructive"
              size="sm"
              onClick={() => void stop()}
              disabled={stopping}
            >
              <Square className="h-3.5 w-3.5" />
              {stopping ? t("status.stopping") : t("action.stop")}
            </Button>
          ) : (
            <Button size="sm" onClick={() => void startFlow()} disabled={busy || stopping}>
              <Play className="h-3.5 w-3.5" />
              {launching ? t("status.launching") : t("action.start")}
            </Button>
          )}
          <Button
            variant="outline"
            size="sm"
            onClick={() => void restart()}
            disabled={!running || busy}
            title={t("action.restart")}
          >
            <RotateCw className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>

      {/* 状态概览 */}
      <Card>
        <CardContent className="flex flex-wrap items-center gap-x-8 gap-y-3 p-4">
          <div className="flex items-center gap-3">
            <StatusBadge status={status} />
            {statusError(status) && (
              <span className="max-w-md truncate text-xs text-destructive" title={statusError(status)}>
                {statusError(status)}
              </span>
            )}
            {launching && (
              <span className="text-xs text-muted-foreground">{t("dashboard.healthChecking")}</span>
            )}
          </div>
          <div className="flex items-center gap-6 text-xs">
            <div>
              <span className="text-muted-foreground">{t("dashboard.runningProfile")}: </span>
              <span className="font-medium">{runningProfile ?? t("dashboard.none")}</span>
            </div>
            <div>
              <span className="text-muted-foreground">{t("dashboard.editingProfile")}: </span>
              <span className="font-medium">{editing || t("dashboard.none")}</span>
            </div>
            <div>
              <span className="text-muted-foreground">{t("dashboard.pid")}: </span>
              <span className="mono">{pid ?? "—"}</span>
            </div>
            <div>
              <span className="text-muted-foreground">{t("dashboard.uptime")}: </span>
              <span className="mono tabular-nums">
                {startedAt ? formatDuration(now - startedAt) : "—"}
              </span>
            </div>
            <div className="mono text-muted-foreground">
              {runningProfileObj
                ? `${runningProfileObj.host}:${runningProfileObj.port}`
                : editingProfile
                  ? `${editingProfile.host}:${editingProfile.port}`
                  : ""}
            </div>
          </div>
        </CardContent>
      </Card>

      {/* 指标卡 */}
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <MetricCard
          icon={Cpu}
          label={t("dashboard.cpu")}
          value={metrics ? formatPercent(metrics.cpu_percent) : "—"}
          sub={t("dashboard.sampleInterval", { n: sampleSeconds })}
          tone={metrics && metrics.cpu_percent >= cpuAlert ? "warning" : "default"}
        />
        <MetricCard
          icon={HardDrive}
          label={t("dashboard.memory")}
          value={metrics ? formatBytes(metrics.mem_bytes) : "—"}
          sub={t("dashboard.lastMinutes", { n: windowMinutes })}
        />
        <MetricCard
          icon={Gauge}
          label={t("dashboard.gpu")}
          value={
            metrics?.gpu_mem_used_bytes != null
              ? formatBytes(metrics.gpu_mem_used_bytes)
              : t("misc.notAvailable")
          }
          sub={
            metrics?.gpu_mem_total_bytes != null
              ? `/ ${formatBytes(metrics.gpu_mem_total_bytes)}`
              : t("dashboard.cpuOnly")
          }
        >
          {gpuAvailable && <Badge tone="muted">{metrics?.gpu_source}</Badge>}
        </MetricCard>
        <MetricCard
          icon={Thermometer}
          label={t("dashboard.gpuUtil")}
          value={
            metrics?.gpu_util_percent != null
              ? formatPercent(metrics.gpu_util_percent)
              : t("misc.notAvailable")
          }
          sub={
            metrics?.gpu_temp_c != null
              ? `${t("dashboard.gpuTemp")} ${metrics.gpu_temp_c}°C`
              : metrics?.gpu_name ?? t("dashboard.cpuOnly")
          }
          tone={
            metrics?.gpu_util_percent != null && metrics.gpu_util_percent >= gpuAlert
              ? "warning"
              : "default"
          }
        />
      </div>

      {/* 告警 */}
      <Card>
        <CardContent className="flex items-center gap-2 p-3 text-xs">
          {alerts.length === 0 ? (
            <>
              <Activity className="h-3.5 w-3.5 text-success" />
              <span className="text-muted-foreground">{t("dashboard.noAlert")}</span>
            </>
          ) : (
            <>
              <AlertTriangle className="h-3.5 w-3.5 text-warning" />
              <span className="text-warning">{alerts.join(" · ")}</span>
            </>
          )}
          <div className="ml-auto flex items-center gap-1">
            {metrics?.gpu_name && (
              <span className="mono mr-2 max-w-56 truncate text-muted-foreground">
                {metrics.gpu_name}
              </span>
            )}
            <Button
              variant="ghost"
              size="sm"
              className="h-6 px-2 text-[11px]"
              disabled={samples.length === 0}
              onClick={() => void exportSamples("csv")}
            >
              {t("dashboard.exportCsv")}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className="h-6 px-2 text-[11px]"
              disabled={samples.length === 0}
              onClick={() => void exportSamples("json")}
            >
              {t("dashboard.exportJson")}
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* 趋势图 */}
      <div className="grid gap-3 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>{t("dashboard.cpu")} · {t("dashboard.trend")}</CardTitle>
          </CardHeader>
          <CardContent className="h-48">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={chartData}>
                <defs>
                  <linearGradient id="cpuFill" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="hsl(var(--primary))" stopOpacity={0.35} />
                    <stop offset="95%" stopColor="hsl(var(--primary))" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
                <XAxis dataKey="label" tick={{ fontSize: 10 }} stroke="hsl(var(--muted-foreground))" minTickGap={40} />
                <YAxis unit="%" domain={[0, 100]} tick={{ fontSize: 10 }} stroke="hsl(var(--muted-foreground))" width={38} />
                <Tooltip
                  contentStyle={{
                    background: "hsl(var(--popover))",
                    border: "1px solid hsl(var(--border))",
                    borderRadius: 8,
                    fontSize: 11,
                  }}
                  formatter={(v) => [`${Number(v).toFixed(1)}%`, t("dashboard.cpu")]}
                />
                <Area
                  type="monotone"
                  dataKey="cpu"
                  stroke="hsl(var(--primary))"
                  fill="url(#cpuFill)"
                  strokeWidth={1.6}
                  isAnimationActive={false}
                  dot={false}
                />
              </AreaChart>
            </ResponsiveContainer>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t("dashboard.memory")} · {t("dashboard.trend")}</CardTitle>
          </CardHeader>
          <CardContent className="h-48">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={chartData}>
                <defs>
                  <linearGradient id="memFill" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="hsl(var(--warning))" stopOpacity={0.35} />
                    <stop offset="95%" stopColor="hsl(var(--warning))" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
                <XAxis dataKey="label" tick={{ fontSize: 10 }} stroke="hsl(var(--muted-foreground))" minTickGap={40} />
                <YAxis unit="MB" tick={{ fontSize: 10 }} stroke="hsl(var(--muted-foreground))" width={48} />
                <Tooltip
                  contentStyle={{
                    background: "hsl(var(--popover))",
                    border: "1px solid hsl(var(--border))",
                    borderRadius: 8,
                    fontSize: 11,
                  }}
                  formatter={(v) => [`${Number(v).toFixed(0)} MB`, t("dashboard.memory")]}
                />
                <Area
                  type="monotone"
                  dataKey="mem"
                  stroke="hsl(var(--warning))"
                  fill="url(#memFill)"
                  strokeWidth={1.6}
                  isAnimationActive={false}
                  dot={false}
                />
              </AreaChart>
            </ResponsiveContainer>
          </CardContent>
        </Card>
      </div>

      {gpuAvailable && (
        <Card>
          <CardHeader>
            <CardTitle>{t("dashboard.gpuUtil")} · {t("dashboard.trend")}</CardTitle>
          </CardHeader>
          <CardContent className="h-44">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
                <XAxis dataKey="label" tick={{ fontSize: 10 }} stroke="hsl(var(--muted-foreground))" minTickGap={40} />
                <YAxis unit="%" domain={[0, 100]} tick={{ fontSize: 10 }} stroke="hsl(var(--muted-foreground))" width={38} />
                <Tooltip
                  contentStyle={{
                    background: "hsl(var(--popover))",
                    border: "1px solid hsl(var(--border))",
                    borderRadius: 8,
                    fontSize: 11,
                  }}
                />
                <Area
                  type="monotone"
                  dataKey="gpuUtil"
                  stroke="hsl(var(--success))"
                  fill="transparent"
                  strokeWidth={1.6}
                  isAnimationActive={false}
                  dot={false}
                />
              </AreaChart>
            </ResponsiveContainer>
          </CardContent>
        </Card>
      )}

      {/* 错误诊断 */}
      {kind === "error" && (
        <Card className="border-destructive/40">
          <CardHeader className="flex-row items-center justify-between">
            <CardTitle className="text-destructive">{t("dashboard.diagnostics")}</CardTitle>
            <div className="flex gap-1">
              {preview && (
                <Button
                  variant="ghost"
                  size="icon"
                  title={t("action.copy")}
                  onClick={() => void navigator.clipboard.writeText(preview.preview)}
                >
                  <Copy className="h-3.5 w-3.5" />
                </Button>
              )}
            </div>
          </CardHeader>
          <CardContent className="space-y-3">
            <p className="text-xs text-destructive">{statusError(status)}</p>
            {diagnosisKey && (
              <div className="rounded-md border border-destructive/30 bg-destructive/5 p-2.5">
                <div className="mb-1 text-[11px] font-medium text-destructive">
                  {t("error.diag.title")}
                </div>
                <p className="text-xs text-foreground/90">{t(diagnosisKey)}</p>
              </div>
            )}
            {preview && (
              <div>
                <div className="mb-1 flex items-center justify-between">
                  <span className="text-[11px] font-medium text-muted-foreground">
                    {t("dashboard.command")}
                  </span>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => void navigator.clipboard.writeText(preview.preview)}
                  >
                    <Copy className="h-3 w-3" />
                  </Button>
                </div>
                <pre className="mono max-h-24 overflow-auto whitespace-pre-wrap break-all rounded-md border border-border bg-muted/40 p-2 text-[11px]">
                  {preview.preview}
                </pre>
              </div>
            )}
            {logLines.length > 0 && (
              <div>
                <div className="mb-1 text-[11px] font-medium text-muted-foreground">
                  {t("dashboard.stderr")}
                </div>
                <pre className="mono max-h-40 overflow-auto whitespace-pre-wrap break-all rounded-md border border-border bg-muted/40 p-2 text-[11px]">
                  {logLines.slice(-200).join("\n")}
                </pre>
              </div>
            )}
          </CardContent>
        </Card>
      )}

      {/* 日志面板 */}
      <Card>
        <CardHeader className="flex-row items-center justify-between">
          <button
            onClick={() => setLogOpen((v) => !v)}
            className="flex items-center gap-2 text-left"
          >
            <Terminal className="h-3.5 w-3.5 text-muted-foreground" />
            <CardTitle>{t("dashboard.log")}</CardTitle>
            <Badge tone="muted">{logLines.length}</Badge>
            <ChevronDown
              className={cn(
                "h-4 w-4 text-muted-foreground transition-transform",
                logOpen && "rotate-180",
              )}
            />
          </button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => void navigator.clipboard.writeText(logLines.join("\n"))}
            disabled={logLines.length === 0}
          >
            <Copy className="h-3.5 w-3.5" />
            {t("action.copy")}
          </Button>
        </CardHeader>
        {logOpen && (
          <CardContent>
            {logLines.length === 0 ? (
              <p className="py-6 text-center text-xs text-muted-foreground">{t("dashboard.noLog")}</p>
            ) : (
              <pre
                ref={logRef}
                className="mono max-h-64 overflow-auto whitespace-pre-wrap break-all rounded-md border border-border bg-muted/30 p-2.5 text-[11px] leading-relaxed"
              >
                {logLines.join("\n")}
              </pre>
            )}
          </CardContent>
        )}
      </Card>

      <Dialog
        open={portInfo !== null}
        onClose={() => setPortInfo(null)}
        title={`${t("error.portInUse")} :${portInfo?.port ?? ""}`}
        description={
          portInfo?.process_name
            ? `${portInfo.process_name} (PID ${portInfo.pid ?? "?"})`
            : t("error.portInUse")
        }
        footer={
          <>
            <Button variant="outline" onClick={() => setPortInfo(null)}>
              {t("action.cancel")}
            </Button>
            {freePort && (
              <Button onClick={() => void startWithPort(freePort)}>
                {t("action.continue")} :{freePort}
              </Button>
            )}
            <Button
              variant="destructive"
              onClick={() => {
                setPortInfo(null);
                if (editingProfile) void start({ ...editingProfile });
              }}
            >
              {t("action.forceStart")}
            </Button>
          </>
        }
      />
    </div>
  );
}
