# Perch

> ネイティブ `llama-server`（llama.cpp）向けの、macOS 風デザインの Windows デスクトップランチャー兼ランタイムガーディアン。

[中文](README.md) · [English](README.en.md) · [한국어](README.ko.md) · [Русский](README.ru.md)

Perch は llama.cpp を同梱・配布しません。お手持ちの `llama-server.exe` と GGUF モデルを用意すれば、パラメータ管理、ワンクリック起動、リアルタイム監視、障害診断、常駐トレイを利用できます。

## 機能

- **バイナリ検出**：exe または release ルートを選択し、CPU / CUDA / Vulkan / SYCL バックエンドとバージョンを自動判定。`--help` を走査してパラメータフォームを生成・キャッシュします。
- **複数プロファイル**：モデルごとに host/port/`-ngl`/スレッド/コンテキスト/追加引数/環境変数/作業ディレクトリを保存。TOML の入出力に対応（モデルパスのマスク可）。
- **推奨値**：GGUF メタデータ（アーキテクチャ、量子化、学習コンテキスト、レイヤー数）と GPU VRAM から `-ngl`、`-c`、`-t`、`--flash-attn` を提案し、ワンクリックで適用。
- **確実な起動**：コンソール非表示で起動し、`/health` 成功後にのみ「実行中」と判定。ポート競合、VRAM 不足、モデル破損、引数不正、権限不足などを分類して表示。
- **リアルタイム監視**：CPU、ワーキングセット、GPU 使用率、VRAM、温度の推移としきい値アラート。CSV / JSON 出力。
- **ログ**：セッションごとにファイル分割。表示・コピー・エクスポート、古いログの自動削除。
- **トレイ常駐**：状態に応じたメニューで起動/停止、プロファイル切替、Web UI を開く。閉じるボタンでトレイに格納可能。
- **単一インスタンス**：再起動時は既存ウィンドウを前面化し、二重起動を防止。
- **ローカル読み取り専用 API**：`GET /status`、`/metrics`、`/profiles`。`127.0.0.1` のみにバインド、任意で Bearer トークン。
- **CLI**：`status` / `start` / `stop` / `export`。
- **中国語・英語 UI**、ライト / ダーク / システムテーマ。

## 必要環境

- Windows 11 x64（Windows 10 21H2 以降も動作する見込み）
- ご自身の `llama-server.exe`（llama.cpp 公式 release または自作ビルド）
- GGUF モデル
- ビルド時：Node.js 22+、pnpm 10+、Rust 1.80+、Visual Studio C++ Build Tools

## クイックスタート

1. Releases から `Perch_x.y.z_x64-setup.exe` をダウンロードしてインストール。
2. 初回起動時に `llama-server.exe`（または llama.cpp 展開フォルダ）と GGUF モデルを選択。
3. 「起動パラメータ」でポートや GPU レイヤーを確認し、「起動」をクリック。
4. 「実行中」になったら「Web UI を開く」で `http://127.0.0.1:<ポート>` を表示。

> 未署名ビルドのため、初回に SmartScreen の警告が出る場合があります。「詳細情報 → 実行」を選択してください。本リポジトリの Releases からのみ入手してください。

## 使い方

### 起動パラメータ

主要項目はフォームで編集できます。「詳細パラメータ」は `llama-server --help` から動的生成され、検索・お気に入りに対応。コマンドプレビューは重複や上書きを警告します。`.bat` を書き出せば Perch なしで実行できます。

### プロファイル

1 プロファイル = 1 モデル + 1 組の設定。同名インポートは自動的に改名されます。マスク出力はモデルパスをプレースホルダーに置換します。

### 監視と診断

ダッシュボードは既定で 1 秒ごと、5 分間のデータを表示。GPU 情報は NVML を優先し、失敗時は `nvidia-smi` にフォールバックします。マルチ GPU 環境では設定で監視対象を選択できます。NVIDIA GPU が無い場合は CPU モードになります。起動失敗時は診断結果、完全なコマンドライン、ログ末尾を表示します。

### トレイと終了

ウィンドウを閉じると既定でトレイに格納されます（設定で変更可）。終了時に llama-server を残すかどうかも選択できます。

## CLI

```powershell
perch.exe --cli status
perch.exe --cli start [--profile <名前>]
perch.exe --cli stop
perch.exe --cli export <ディレクトリ>
```

`status/start/stop` は名前付きパイプで実行中の GUI と通信します。`start` は GUI 未起動時に起動を試みます。出力は JSON です。`export` はオフラインで設定・ヘルプキャッシュ・ログを書き出します。

## ローカル読み取り専用 API

設定で有効化し、ポートを指定します（既定 `3999`、`127.0.0.1` のみ）。トークン設定時は次を付与します。

```http
Authorization: Bearer <token>
```

```powershell
curl.exe http://127.0.0.1:3999/status -H "Authorization: Bearer <token>"
curl.exe http://127.0.0.1:3999/metrics -H "Authorization: Bearer <token>"
curl.exe http://127.0.0.1:3999/profiles -H "Authorization: Bearer <token>"
```

`/profiles` の環境変数値はマスクされます。

## データとプライバシー

- 設定：`%USERPROFILE%\.perch\config.toml`
- ヘルプキャッシュ：`%USERPROFILE%\.perch\help_cache.json`
- ログ：`%USERPROFILE%\.perch\logs\server_<timestamp>.log`
- すべてローカル保存です。テレメトリやクラウド通信は行いません。

## ソースからビルド

```powershell
git clone https://github.com/Jack-Sai/perch.git
cd perch
pnpm install
pnpm tauri build
```

開発：

```powershell
pnpm tauri dev
```

検証：

```powershell
pnpm build
cd src-tauri
cargo test
cargo clippy
```

## サードパーティ依存とライセンス

主な依存（完全な一覧は `src-tauri/Cargo.lock` と `pnpm-lock.yaml`）：

| 依存 | 用途 | ライセンス |
| --- | --- | --- |
| Tauri 2 / tauri-plugin-* | デスクトップ基盤、ダイアログ、通知、自動起動、リンク | MIT / Apache-2.0 |
| axum / tokio | ローカル読み取り専用 API | MIT |
| serde / serde_json / toml | 設定とシリアライズ | MIT / Apache-2.0 |
| sysinfo | プロセス・メモリ監視 | MIT |
| nvml-wrapper | NVML による GPU 指標 | MIT / Apache-2.0 |
| windows-sys | Win32 API（名前付きパイプ、ミューテックス、コンソール） | MIT / Apache-2.0 |
| React / Vite | フロントエンド | MIT |
| Tailwind CSS | スタイル | MIT |
| Recharts | グラフ | MIT |
| lucide-react | アイコン | ISC |

GPU 情報は NVML（存在すればシステムの `nvml.dll` を実行時ロード）を優先し、失敗時は `nvidia-smi` を使用します。NVML の静的リンクや NVIDIA コンポーネントの同梱は行いません。llama.cpp とモデルは同梱されません。各ライセンスを遵守してください。

## コントリビューション

[CONTRIBUTING.md](CONTRIBUTING.md) と [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) をご覧ください。変更履歴は [CHANGELOG.md](CHANGELOG.md)。

## ライセンス

[MIT](LICENSE) © 2026 Jack-Sai
