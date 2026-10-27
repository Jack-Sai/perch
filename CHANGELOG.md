# Changelog

本文件记录 Perch 的所有重要变更，格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本号遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/)。

## [Unreleased]

### Planned

- 多 GPU 选择与主卡指标切换
- 启动器自身运行日志
- 首次隐藏到托盘提示气泡

## [0.1.0] - 2026-10-27

首个可用版本。

### Added

- Tauri 2 + React/TypeScript 桌面应用骨架，macOS 风格自定义标题栏、侧边栏、浅色/深色/跟随系统主题。
- llama-server 二进制发现与后端识别（CPU / CUDA / Vulkan / SYCL），`--help` 扫描、参数解析与本地缓存。
- 常用参数表单、基于 `--help` 的动态高级参数、搜索、收藏、自定义参数、实时命令预览、重复/覆盖警告与 `.bat` 导出。
- 多套配置方案，TOML 导入/导出，支持模型路径脱敏。
- 进程生命周期管理：无黑窗启动、`/health` 健康检查、优雅停止与强制终止回退、崩溃检测与系统通知。
- 首次使用向导（语言、主题、二进制、模型、端口与并行）。
- 仪表盘：CPU/内存/GPU/显存/温度指标、Recharts 趋势图、LTTB 降采样、可配置阈值告警、CSV/JSON 导出。
- 启动失败诊断：端口占用、显存不足、模型无效、参数非法、权限不足、超时等分类，展示完整命令行与日志尾部。
- 会话日志：查看、复制、导出、保留期自动清理，兼容非 UTF-8 输出。
- 系统托盘：状态图标（灰/绿/红）、状态感知菜单、启动/停止、配置切换、打开窗口与 Web UI。
- 单实例：全局互斥量 + 命名管道聚焦，重复启动不产生第二个守护进程。
- 本地只读 API：`/status`、`/metrics`、`/profiles`，仅绑定 `127.0.0.1`，可选 Bearer Token，环境变量掩码。
- CLI：`--cli status|start|stop|export`，通过命名管道与 GUI 通信，JSON 输出。
- 基于 GGUF 元数据与 GPU 显存的推荐参数（`-ngl`、`-c`、`-t`、`--flash-attn`）。
- 中英双语界面与五语 README（中、英、日、韩、俄）。
- 单元测试：参数拆分与覆盖判定、`--help` 解析、GGUF 元数据解析、API 鉴权与环境变量掩码；可选真实二进制扫描集成测试。

### Known limitations

- GPU 指标通过 `nvidia-smi` 获取，尚未接入 NVML。
- 仅支持 NVIDIA GPU 指标；AMD/Intel 设备以 CPU 模式运行。
- 应用未做代码签名，首次运行可能出现 SmartScreen 提示。
- 不包含 llama.cpp 与模型文件。

[Unreleased]: https://github.com/Jack-Sai/perch/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Jack-Sai/perch/releases/tag/v0.1.0
