import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "./api";
import { useI18n, type Lang } from "./i18n";
import { useTheme, type Theme } from "./theme";
import { useToast } from "./toast";
import type {
  AppConfig,
  BinaryInfo,
  GlobalConfig,
  Profile,
  ServerStatus,
  StatusEvent,
} from "./types";
import { statusKind } from "./types";

interface StoreValue {
  cfg: AppConfig | null;
  loading: boolean;
  editing: string;
  setEditing: (name: string) => void;
  editingProfile: Profile | null;
  runningProfile: string | null;
  status: ServerStatus;
  pid: number | null;
  startedAt: number | null;
  binary: BinaryInfo | null;
  wizardOpen: boolean;
  setWizardOpen: (open: boolean) => void;
  reload: () => Promise<void>;
  saveCfg: (next: AppConfig, opts?: { silent?: boolean }) => Promise<void>;
  updateGlobal: (patch: Partial<GlobalConfig>) => void;
  updateProfile: (name: string, patch: Partial<Profile>) => void;
  addProfile: (name: string) => Promise<void>;
  removeProfile: (name: string) => Promise<void>;
  selectProfile: (name: string) => Promise<void>;
  refreshBinary: () => Promise<void>;
  setBinary: (b: BinaryInfo | null) => void;
  start: (profile?: Profile) => Promise<boolean>;
  stop: () => Promise<void>;
  restart: () => Promise<void>;
  openWebui: () => Promise<void>;
  applyPreferences: (g: GlobalConfig) => void;
}

const StoreContext = createContext<StoreValue | null>(null);

export function StoreProvider({ children }: { children: React.ReactNode }) {
  const { t, setLang } = useI18n();
  const { setTheme } = useTheme();
  const { toast } = useToast();

  const [cfg, setCfg] = useState<AppConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [editing, setEditingState] = useState("");
  const [binary, setBinary] = useState<BinaryInfo | null>(null);
  const [status, setStatus] = useState<ServerStatus>("idle");
  const [runningProfile, setRunningProfile] = useState<string | null>(null);
  const [pid, setPid] = useState<number | null>(null);
  const [startedAt, setStartedAt] = useState<number | null>(null);
  const [wizardOpen, setWizardOpen] = useState(false);
  const cfgRef = useRef<AppConfig | null>(null);
  const editingRef = useRef("");

  cfgRef.current = cfg;
  editingRef.current = editing;

  const applyPreferences = useCallback(
    (g: GlobalConfig) => {
      if (g.language === "zh-CN" || g.language === "en-US") setLang(g.language as Lang);
      if (g.theme === "light" || g.theme === "dark" || g.theme === "system")
        setTheme(g.theme as Theme);
    },
    [setLang, setTheme],
  );

  const chooseEditing = useCallback((c: AppConfig) => {
    const last = c.profiles.find((p) => p.name === c.global.last_profile);
    const name = last?.name ?? c.profiles[0]?.name ?? "";
    setEditingState(name);
    editingRef.current = name;
  }, []);

  const reload = useCallback(async () => {
    try {
      const next = await api.loadConfig();
      setCfg(next);
      chooseEditing(next);
      applyPreferences(next.global);
      if (!next.global.first_run_done) setWizardOpen(true);
    } catch (e) {
      toast(String(e), "error");
    }
  }, [applyPreferences, chooseEditing, toast]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    (async () => {
      await reload();
      try {
        const b = await api.inspectBinary();
        if (!disposed) setBinary(b);
      } catch {
        if (!disposed) setBinary(null);
      }
      try {
        const s = await api.getServerStatus();
        if (!disposed) {
          setStatus(s);
          if (statusKind(s) === "running") setStartedAt((prev) => prev ?? Date.now());
        }
      } catch {
        /* ignore */
      }
      if (disposed) return;
      setLoading(false);

      unlisten = await listen<StatusEvent>("perch-status", async (event) => {
        const ev = event.payload;
        if (ev.status === "running") {
          setStatus("running");
          setStartedAt((prev) => prev ?? Date.now());
        } else if (ev.status === "error") {
          setStartedAt(null);
          try {
            setStatus(await api.getServerStatus());
          } catch {
            setStatus({ error: "unknown" });
          }
        } else {
          setStatus(ev.status);
          if (ev.status === "idle") setStartedAt(null);
        }
        setPid(ev.pid);
        setRunningProfile(ev.running_profile);
      });
    })();

    return () => {
      disposed = true;
      unlisten?.();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const saveCfg = useCallback(
    async (next: AppConfig, opts?: { silent?: boolean }) => {
      try {
        await api.saveConfig(next);
        setCfg(next);
        applyPreferences(next.global);
        if (!opts?.silent) toast(t("action.saved"), "success");
      } catch (e) {
        toast(String(e), "error");
        throw e;
      }
    },
    [applyPreferences, t, toast],
  );

  const updateGlobal = useCallback((patch: Partial<GlobalConfig>) => {
    setCfg((prev) => (prev ? { ...prev, global: { ...prev.global, ...patch } } : prev));
  }, []);

  const updateProfile = useCallback((name: string, patch: Partial<Profile>) => {
    setCfg((prev) =>
      prev
        ? {
            ...prev,
            profiles: prev.profiles.map((p) => (p.name === name ? { ...p, ...patch } : p)),
          }
        : prev,
    );
  }, []);

  const setEditing = useCallback((name: string) => {
    setEditingState(name);
    editingRef.current = name;
  }, []);

  const addProfile = useCallback(
    async (name: string) => {
      try {
        const next = await api.createProfile(name);
        setCfg(next);
        setEditing(name);
        editingRef.current = name;
        toast(t("profiles.created"), "success");
      } catch (e) {
        toast(String(e), "error");
      }
    },
    [t, toast],
  );

  const removeProfile = useCallback(
    async (name: string) => {
      try {
        const next = await api.deleteProfile(name);
        setCfg(next);
        if (editingRef.current === name) chooseEditing(next);
        toast(t("profiles.deleted"), "success");
      } catch (e) {
        toast(String(e), "error");
      }
    },
    [chooseEditing, t, toast],
  );

  const selectProfile = useCallback(
    async (name: string) => {
      setEditing(name);
      const current = cfgRef.current;
      if (current) {
        const next = { ...current, global: { ...current.global, last_profile: name } };
        setCfg(next);
        try {
          await api.setLastProfile(name);
        } catch {
          /* non-fatal */
        }
      }
    },
    [],
  );

  const refreshBinary = useCallback(async () => {
    try {
      const b = await api.inspectBinary();
      setBinary(b);
    } catch (e) {
      setBinary(null);
      toast(String(e), "error");
    }
  }, [toast]);

  const start = useCallback(
    async (profile?: Profile) => {
      const current = cfgRef.current;
      const target =
        profile ?? current?.profiles.find((p) => p.name === editingRef.current) ?? null;
      if (!target) {
        toast("没有可用的配置", "error");
        return false;
      }
      try {
        await api.saveConfig(current!);
        const s = await api.startServer(target);
        setStatus(s);
        setRunningProfile(target.name);
        return true;
      } catch (e) {
        toast(String(e), "error");
        return false;
      }
    },
    [toast],
  );

  const stop = useCallback(async () => {
    try {
      const s = await api.stopServer();
      setStatus(s);
    } catch (e) {
      toast(String(e), "error");
    }
  }, [toast]);

  const restart = useCallback(async () => {
    await stop();
    for (let i = 0; i < 120; i++) {
      await new Promise((r) => setTimeout(r, 250));
      try {
        const s = await api.getServerStatus();
        if (statusKind(s) === "idle" || statusKind(s) === "error") break;
      } catch {
        break;
      }
    }
    await start();
  }, [start, stop]);

  const openWebui = useCallback(async () => {
    const current = cfgRef.current;
    const running = current?.profiles.find((p) => p.name === runningProfile);
    const edit = current?.profiles.find((p) => p.name === editingRef.current);
    const port = running?.port ?? edit?.port ?? 8080;
    try {
      await openUrl(`http://127.0.0.1:${port}`);
    } catch (e) {
      toast(String(e), "error");
    }
  }, [runningProfile, toast]);

  const editingProfile = useMemo(
    () => cfg?.profiles.find((p) => p.name === editing) ?? null,
    [cfg, editing],
  );

  const value = useMemo<StoreValue>(
    () => ({
      cfg,
      loading,
      editing,
      setEditing,
      editingProfile,
      runningProfile,
      status,
      pid,
      startedAt,
      binary,
      wizardOpen,
      setWizardOpen,
      reload,
      saveCfg,
      updateGlobal,
      updateProfile,
      addProfile,
      removeProfile,
      selectProfile,
      refreshBinary,
      setBinary,
      start,
      stop,
      restart,
      openWebui,
      applyPreferences,
    }),
    [
      cfg,
      loading,
      editing,
      setEditing,
      editingProfile,
      runningProfile,
      status,
      pid,
      startedAt,
      binary,
      wizardOpen,
      reload,
      saveCfg,
      updateGlobal,
      updateProfile,
      addProfile,
      removeProfile,
      selectProfile,
      refreshBinary,
      start,
      stop,
      restart,
      openWebui,
      applyPreferences,
    ],
  );

  return <StoreContext.Provider value={value}>{children}</StoreContext.Provider>;
}

export function useStore() {
  const ctx = useContext(StoreContext);
  if (!ctx) throw new Error("useStore must be used within StoreProvider");
  return ctx;
}
