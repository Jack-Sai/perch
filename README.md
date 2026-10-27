# Perch

> macOS 风格的 Windows 桌面图形启动器与运行时守护程序，用于原生 `llama-server`（llama.cpp）。

[English](README.en.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Русский](README.ru.md)

Perch 不捆绑、不分发 llama.cpp。你只需提供自己的 `llama-server.exe` 与 GGUF 模型，即可获得参数管理、一键启动、实时监控、异常诊断与托盘常驻能力。

## 功能

- **二进制识别**：选择 exe 或 release 根目录，自动识别 CPU / CUDA / Vulkan / SYCL 后端与版本，并扫描 `--help` 生成参数表单与缓存。
- **多配置方案**：每套模型独立保存 host/port/`-ngl`/线程/上下文/额外参数/环境变量/工作目录，支持 TOML 导入导出（可脱敏模型路径）。
- **智能推荐**：解析 GGUF 元数据（架构、量化、训练上下文、层数）并结合 GPU 显存，一键应用 `-ngl`、`-c`、`-t`、`--flash-attn` 建议。
- **可靠启动**：无黑窗启动，等待 `/health` 通过后才判定成功；启动失败自动区分端口占用、显存不足、模型损坏、参数非法、权限不足等原因。
- **实时监控**：进程 CPU / 内存工作集 / GPU 利用率 / 显存 / 温度趋势，阈值告警，数据可导出 CSV / JSON。
- **日志**：按会话分隔日志，支持查看、复制与导出，自动清理过期日志。
- **托盘常驻**：状态感知菜单，直接启动/停止、切换配置、打开 Web UI；关闭窗口可隐藏到托盘。
- **单实例**：重复启动会聚焦已有窗口，避免多个守护进程争抢端口。
- **本地只读 API**：`GET /status`、`/metrics`、`/profiles`，仅监听 `127.0.0.1`，可选 Bearer Token。
- **CLI**：`status` / `start` / `stop` / `export`，便于脚本集成。
- **中英双语界面**，浅色 / 深色 / 跟随系统主题。

## 系统要求

- Windows 11 x64（Windows 10 21H2+ 理论可用）
- 自带 `llama-server.exe`（llama.cpp 官方 release 或自行编译）
- GGUF 模型文件
- 构建需要：Node.js 22+、pnpm 10+、Rust 1.80+、Visual Studio C++ Build Tools

## 快速开始

1. 从 Release 下载 `Perch_x.y.z_x64-setup.exe` 并安装。
2. 首次启动按向导选择 `llama-server.exe`（或 llama.cpp 解压目录）与 GGUF 模型。
3. 在「启动参数」中确认端口、GPU 层数等，点击「启动」。
4. 状态变为「运行中」后，通过「打开 Web UI」访问 `http://127.0.0.1:<端口>`。

> 未签名应用：首次运行可能出现 Windows SmartScreen 提示，选择「更多信息 → 仍要运行」即可。请只从本仓库 Releases 下载。

## 使用说明

### 启动参数

常用参数提供表单；「高级参数」由 `llama-server --help` 动态生成，可搜索与收藏。命令行预览会实时显示，并提示参数重复或被覆盖的情况。导出 `.bat` 可脱离 Perch 运行。

### 配置方案

每套方案对应一个模型与一组参数。导入 TOML 时同名配置会自动重命名。脱敏导出会把模型路径替换为占位符，适合分享。

### 监控与诊断

仪表盘每 N 秒采样（默认 1 秒，窗口 5 分钟）。GPU 指标优先通过 `nvidia-smi` 获取，未检测到 NVIDIA GPU 时自动进入 CPU 模式。启动失败时展示诊断结论、完整命令行与日志尾部。

### 托盘与关闭行为

关闭主窗口默认隐藏到托盘；可在设置中改为退出。退出时可选择是否保留 llama-server 进程。

## CLI

```powershell
perch.exe --cli status
perch.exe --cli start [--profile <名称>]
perch.exe --cli stop
perch.exe --cli export <目录>
```

`status/start/stop` 通过命名管道与正在运行的 GUI 通信；`start` 在 GUI 未运行时会尝试拉起。输出为 JSON。`export` 可离线执行，导出配置、帮助缓存与日志。

## 本地只读 API

在设置中开启并配置端口（默认 `3999`，仅绑定 `127.0.0.1`）。若设置了 Token，请携带：

```http
Authorization: Bearer <token>
```

```powershell
curl.exe http://127.0.0.1:3999/status -H "Authorization: Bearer <token>"
curl.exe http://127.0.0.1:3999/metrics -H "Authorization: Bearer <token>"
curl.exe http://127.0.0.1:3999/profiles -H "Authorization: Bearer <token>"
```

环境变量值在 `/profiles` 中会被掩码。

## 数据与隐私

- 配置：`%USERPROFILE%\.perch\config.toml`
- 帮助缓存：`%USERPROFILE%\.perch\help_cache.json`
- 日志：`%USERPROFILE%\.perch\logs\server_<timestamp>.log`
- 所有数据仅保存在本机；Perch 不发起任何遥测或云端请求。

## 从源码构建

```powershell
git clone https://github.com/Jack-Sai/perch.git
cd perch
pnpm install
pnpm tauri build
```

开发模式：

```powershell
pnpm tauri dev
```

测试与检查：

```powershell
pnpm build
cd src-tauri
cargo test
cargo clippy
```

## 第三方依赖与许可

主要依赖（完整清单见 `src-tauri/Cargo.lock` 与 `pnpm-lock.yaml`）：

| 依赖 | 用途 | 许可证 |
| --- | --- | --- |
| Tauri 2 / tauri-plugin-* | 桌面框架、对话框、通知、自启动、打开链接 | MIT / Apache-2.0 |
| axum / tokio | 本地只读 API | MIT |
| serde / serde_json / toml | 配置与序列化 | MIT / Apache-2.0 |
| sysinfo | 进程与内存监控 | MIT |
| windows-sys | Win32 API（命名管道、互斥量、控制台） | MIT / Apache-2.0 |
| React / Vite | 前端 | MIT |
| Tailwind CSS | 样式 | MIT |
| Recharts | 图表 | MIT |
| lucide-react | 图标 | ISC |

GPU 数据通过系统 `nvidia-smi` 获取；Perch 不静态链接 NVML，也不分发 NVIDIA 驱动组件。llama.cpp 及其模型不随本项目分发，相关许可证由用户自行遵守。

## 贡献

请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 与 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)。变更记录见 [CHANGELOG.md](CHANGELOG.md)。

## 许可证

[MIT](LICENSE) © 2026 Jack-Sai
