export interface GlobalConfig {
  language: string;
  theme: string;
  last_profile: string;
  autostart: boolean;
  close_to_tray: boolean;
  exit_keep_server: boolean;
  monitor_window_minutes: number;
  monitor_sample_seconds: number;
  binary_path: string;
  api_enabled: boolean;
  api_port: number;
  api_token: string;
  first_run_done: boolean;
}

export interface Profile {
  name: string;
  model_path: string;
  host: string;
  port: number;
  ngl: number;
  threads: number;
  ctx_size: number;
  flash_attn: boolean;
  extra_params: string;
  work_dir: string;
  env_vars: string[];
}

export interface AppConfig {
  global: GlobalConfig;
  profiles: Profile[];
}

export interface HelpArg {
  name: string;
  short: string;
  value: string;
  description: string;
  takes_value: boolean;
}

export interface BinaryInfo {
  path: string;
  backend: string;
  version: string;
  help_args: HelpArg[];
}

export interface CommandPreview {
  args: string[];
  preview: string;
  warnings: string[];
  overridden: string[];
  duplicated: string[];
}

export type ServerStatus =
  | "idle"
  | "launching"
  | "running"
  | "stopping"
  | { error: string };

export interface StatusEvent {
  status: "idle" | "launching" | "running" | "stopping" | "error";
  visual: "green" | "red" | "gray";
  pid: number | null;
  running_profile: string | null;
}

export interface Metrics {
  running: boolean;
  pid: number | null;
  cpu_percent: number;
  mem_bytes: number;
  gpu_util_percent: number | null;
  gpu_mem_used_bytes: number | null;
  gpu_mem_total_bytes: number | null;
  gpu_temp_c: number | null;
  gpu_name: string | null;
  gpu_source: string | null;
}

export function statusKind(s: ServerStatus): "idle" | "launching" | "running" | "stopping" | "error" {
  if (typeof s === "string") return s;
  return "error";
}

export function statusError(s: ServerStatus): string {
  return typeof s === "string" ? "" : s.error;
}

export function defaultProfile(name = "default"): Profile {
  return {
    name,
    model_path: "",
    host: "127.0.0.1",
    port: 8080,
    ngl: -1,
    threads: 0,
    ctx_size: 0,
    flash_attn: false,
    extra_params: "",
    work_dir: "",
    env_vars: [],
  };
}
