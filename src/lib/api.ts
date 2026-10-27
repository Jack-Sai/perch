import { invoke } from "@tauri-apps/api/core";
import type {
  ApiStatus,
  AppConfig,
  BinaryInfo,
  CommandPreview,
  Metrics,
  Profile,
  Recommendation,
  ServerStatus,
} from "./types";

export interface PortStatus {
  port: number;
  available: boolean;
  pid: number | null;
  process_name: string | null;
}

export const api = {
  loadConfig: () => invoke<AppConfig>("load_config_cmd"),
  saveConfig: (cfg: AppConfig) => invoke<void>("save_config_cmd", { cfg }),
  createProfile: (name: string) => invoke<AppConfig>("create_profile", { name }),
  deleteProfile: (name: string) => invoke<AppConfig>("delete_profile", { name }),
  setLastProfile: (name: string) => invoke<AppConfig>("set_last_profile", { name }),
  exportConfig: (path: string, maskModel: boolean) =>
    invoke<void>("export_config_cmd", { path, maskModel }),
  importConfig: (path: string) => invoke<AppConfig>("import_config_cmd", { path }),

  detectBinary: () => invoke<BinaryInfo>("detect_binary_cmd"),
  resolveBinary: (input: string) => invoke<BinaryInfo>("resolve_binary_cmd", { input }),
  inspectBinary: () => invoke<BinaryInfo | null>("inspect_binary_cmd"),
  rescanHelp: (force: boolean) => invoke<BinaryInfo>("rescan_help_cmd", { force }),

  buildCommand: (profile: Profile) => invoke<CommandPreview>("build_command_cmd", { profile }),
  exportBat: (profile: Profile, path: string, absolute: boolean) =>
    invoke<void>("export_bat_cmd", { profile, path, absolute }),
  startServer: (profile: Profile) => invoke<ServerStatus>("start_server_cmd", { profile }),
  stopServer: () => invoke<ServerStatus>("stop_server_cmd"),
  getServerStatus: () => invoke<ServerStatus>("get_server_status_cmd"),
  readServerLog: (tail: number) => invoke<string[]>("read_server_log_cmd", { tail }),
  exportServerLog: (path: string) => invoke<void>("export_server_log_cmd", { path }),
  getMetrics: () => invoke<Metrics>("get_metrics_cmd"),

  checkPort: (port: number) => invoke<PortStatus>("check_port_cmd", { port }),
  findFreePort: (start: number) => invoke<number>("find_free_port_cmd", { start }),
  saveTextFile: (path: string, contents: string) =>
    invoke<void>("save_text_file_cmd", { path, contents }),
  recommend: (profile: Profile) => invoke<Recommendation>("recommend_cmd", { profile }),
  apiStatus: () => invoke<ApiStatus>("api_status_cmd"),
};
