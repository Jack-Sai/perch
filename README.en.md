# Perch

> A macOS-flavored Windows desktop launcher and runtime guardian for native `llama-server` (llama.cpp).

[中文](README.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Русский](README.ru.md)

Perch does not bundle or distribute llama.cpp. Bring your own `llama-server.exe` and GGUF model to get parameter management, one-click launch, live monitoring, failure diagnosis, and tray residency.

## Features

- **Binary detection**: pick the exe or a release root; auto-detect CPU / CUDA / Vulkan / SYCL backend and version, scan `--help` to build the parameter form, and cache the result.
- **Multiple profiles**: per-model host/port/`-ngl`/threads/context/extra args/env/workdir, with TOML import/export (model path can be masked).
- **Recommendations**: parse GGUF metadata (architecture, quantization, trained context, layer count) with GPU VRAM to suggest `-ngl`, `-c`, `-t`, and `--flash-attn` in one click.
- **Reliable launch**: no console window; success only after `/health` passes; failures classified as port conflict, out of VRAM, invalid model, bad argument, permission, and more.
- **Live monitoring**: CPU, working set, GPU utilization, VRAM, and temperature trends, multi-GPU selection, configurable alerts, and CSV / JSON export.
- **Logs**: one file per session, view/copy/export, automatic retention cleanup.
- **Tray**: status-aware menu to start/stop, switch profiles, and open the Web UI; close-to-tray supported.
- **Single instance**: relaunching focuses the existing window instead of spawning a second daemon.
- **Local read-only API**: `GET /status`, `/metrics`, `/profiles`, bound to `127.0.0.1` only, optional Bearer token.
- **CLI**: `status` / `start` / `stop` / `export` for scripting.
- **Chinese/English UI**, light / dark / system themes.

## Requirements

- Windows 11 x64 (Windows 10 21H2+ should work)
- Your own `llama-server.exe` (official llama.cpp release or self-built)
- A GGUF model
- To build: Node.js 22+, pnpm 10+, Rust 1.80+, Visual Studio C++ Build Tools

## Quick start

1. Download `Perch_x.y.z_x64-setup.exe` from Releases and install.
2. On first run, pick `llama-server.exe` (or the extracted llama.cpp folder) and a GGUF model.
3. Review port and GPU layers under **Parameters**, then click **Start**.
4. Once **Running**, click **Open Web UI** to visit `http://127.0.0.1:<port>`.

> Unsigned build: SmartScreen may warn on first launch. Choose **More info → Run anyway**. Download only from this repository's Releases.

## Usage

### Parameters

Common options use a form; **Advanced** is generated from `llama-server --help` and supports search and favorites. A live command preview warns about duplicates and overridden values. Export a `.bat` to run without Perch.

### Profiles

Each profile maps one model to one set of options. Imported profiles with duplicate names are renamed automatically. Masked export replaces model paths with a placeholder for safe sharing.

### Monitoring and diagnosis

The dashboard samples every N seconds (default 1s, 5-minute window). GPU metrics come from NVML when available and fall back to `nvidia-smi`; on multi-GPU systems you can pick the monitored GPU in Settings. Without an NVIDIA GPU Perch runs in CPU mode. On failure you get a diagnosis, the full command line, and the log tail.

### Tray and close behavior

Closing the window hides to tray by default; configurable in Settings. On exit you may keep llama-server running.

## CLI

```powershell
perch.exe --cli status
perch.exe --cli start [--profile <name>]
perch.exe --cli stop
perch.exe --cli export <dir>
```

`status/start/stop` talk to the running GUI over a named pipe; `start` tries to launch the GUI when needed. Output is JSON. `export` works offline and exports config, help cache, and logs.

## Local read-only API

Enable it in Settings and choose a port (default `3999`, bound to `127.0.0.1`). With a token configured, send:

```http
Authorization: Bearer <token>
```

```powershell
curl.exe http://127.0.0.1:3999/status -H "Authorization: Bearer <token>"
curl.exe http://127.0.0.1:3999/metrics -H "Authorization: Bearer <token>"
curl.exe http://127.0.0.1:3999/profiles -H "Authorization: Bearer <token>"
```

Environment values are masked in `/profiles`.

## Data and privacy

- Config: `%USERPROFILE%\.perch\config.toml`
- Help cache: `%USERPROFILE%\.perch\help_cache.json`
- Logs: `%USERPROFILE%\.perch\logs\server_<timestamp>.log`
- All data stays local. Perch sends no telemetry or cloud requests.

## Build from source

```powershell
git clone https://github.com/Jack-Sai/perch.git
cd perch
pnpm install
pnpm tauri build
```

Development:

```powershell
pnpm tauri dev
```

Checks:

```powershell
pnpm build
cd src-tauri
cargo test
cargo clippy
```

## Third-party dependencies and licenses

Main dependencies (full lists in `src-tauri/Cargo.lock` and `pnpm-lock.yaml`):

| Dependency | Purpose | License |
| --- | --- | --- |
| Tauri 2 / tauri-plugin-* | Desktop shell, dialogs, notifications, autostart, opener | MIT / Apache-2.0 |
| axum / tokio | Local read-only API | MIT |
| serde / serde_json / toml | Config and serialization | MIT / Apache-2.0 |
| sysinfo | Process and memory monitoring | MIT |
| nvml-wrapper | GPU metrics via NVML | MIT / Apache-2.0 |
| windows-sys | Win32 APIs (named pipe, mutex, console) | MIT / Apache-2.0 |
| React / Vite | Frontend | MIT |
| Tailwind CSS | Styling | MIT |
| Recharts | Charts | MIT |
| lucide-react | Icons | ISC |

GPU data is read via NVML (loading the system `nvml.dll` at runtime when present), with `nvidia-smi` as fallback; Perch does not statically link NVML or ship NVIDIA components. llama.cpp and models are not distributed with this project; comply with their respective licenses.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Changes are tracked in [CHANGELOG.md](CHANGELOG.md).

## License

[MIT](LICENSE) © 2026 Jack-Sai
