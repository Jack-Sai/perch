# Contributing to Perch

感谢你愿意改进 Perch。本文说明本地开发、代码风格与提交要求。

## 开发环境

- Windows 11 x64
- Node.js 22+ 与 pnpm 10+
- Rust 1.80+
- Visual Studio C++ Build Tools（MSVC）
- WebView2 Runtime（Windows 11 自带）

```powershell
pnpm install
pnpm tauri dev
```

前端类型检查与构建：

```powershell
pnpm build
```

Rust 检查与测试（在 `src-tauri` 下执行）：

```powershell
cargo fmt
cargo clippy --all-targets
cargo test
```

可选的真实二进制集成测试：

```powershell
$env:PERCH_TEST_BINARY = "D:\llama.cpp\llama-server.exe"
cargo test -- --ignored
```

## 分支与提交

- 从 `main` 创建功能分支，例如 `feat/multi-gpu`、`fix/help-parser`。
- 每个独立功能或修复单独提交，避免把无关改动混在一起。
- 提交信息使用中文或英文均可，建议采用 Conventional Commits 前缀：
  - `feat: 新增 ...`
  - `fix: 修复 ...`
  - `docs: 更新 ...`
  - `test: 补充 ...`
  - `chore: 调整 ...`
- 提交前确保上述检查通过，并只暂存本次相关文件。

## 代码约定

- 后端位于 `src-tauri/src`，前端位于 `src`。
- Rust 使用 `cargo fmt` 默认风格；错误信息面向用户时使用中文，并在前端 i18n 中提供英文对应。
- 所有新增界面文案必须同时添加 `zh-CN` 与 `en-US` 键（`src/lib/i18n.tsx`）。
- 配置结构新增字段必须提供默认值，保持旧配置可读（`#[serde(default)]` 与 `Default`）。
- 涉及 Win32 调用时集中封装，避免在 UI 线程执行阻塞操作。
- 不引入遥测、云端请求或未声明的网络访问。
- 不提交密钥、令牌、模型文件或本机绝对路径。

## Pull Request

请说明：

1. 变更目的与用户可见行为。
2. 复现步骤或使用场景。
3. 已执行的检查（`pnpm build`、`cargo test` 等）。
4. 如涉及界面，附截图或简短录屏。

维护者可能会要求补充测试或拆分提交。
