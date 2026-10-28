# Perch

> 네이티브 `llama-server`(llama.cpp)를 위한 macOS 스타일 Windows 데스크톱 런처 겸 런타임 가디언.

[中文](README.md) · [English](README.en.md) · [日本語](README.ja.md) · [Русский](README.ru.md)

Perch는 llama.cpp를 포함하거나 배포하지 않습니다. 사용자의 `llama-server.exe`와 GGUF 모델을 준비하면 파라미터 관리, 원클릭 실행, 실시간 모니터링, 장애 진단, 트레이 상주 기능을 사용할 수 있습니다.

## 기능

- **바이너리 감지**: exe 또는 release 루트를 선택하면 CPU / CUDA / Vulkan / SYCL 백엔드와 버전을 자동 감지하고, `--help`를 스캔해 파라미터 폼을 생성·캐시합니다.
- **다중 프로필**: 모델별 host/port/`-ngl`/스레드/컨텍스트/추가 인자/환경 변수/작업 디렉터리를 저장하고 TOML 가져오기·내보내기(모델 경로 마스킹)를 지원합니다.
- **권장 값**: GGUF 메타데이터(아키텍처, 양자화, 학습 컨텍스트, 레이어 수)와 GPU VRAM을 바탕으로 `-ngl`, `-c`, `-t`, `--flash-attn`을 제안하고 한 번에 적용합니다.
- **안정적인 실행**: 콘솔 창 없이 실행하고 `/health` 통과 후에만 성공으로 판정합니다. 포트 충돌, VRAM 부족, 모델 손상, 잘못된 인자, 권한 부족 등을 구분해 표시합니다.
- **실시간 모니터링**: CPU, 작업 집합, GPU 사용률, VRAM, 온도 추이와 임계값 경고. CSV / JSON 내보내기.
- **로그**: 세션별 파일 분리, 보기·복사·내보내기, 오래된 로그 자동 정리.
- **트레이 상주**: 상태에 맞는 메뉴로 시작/중지, 프로필 전환, Web UI 열기. 창 닫기 시 트레이로 숨김 지원.
- **단일 인스턴스**: 다시 실행하면 기존 창을 앞으로 가져와 중복 실행을 방지합니다.
- **로컬 읽기 전용 API**: `GET /status`, `/metrics`, `/profiles`. `127.0.0.1`에만 바인딩하며 선택적 Bearer 토큰 지원.
- **CLI**: `status` / `start` / `stop` / `export`.
- **중국어·영어 UI**, 라이트 / 다크 / 시스템 테마.

## 요구 사항

- Windows 11 x64 (Windows 10 21H2 이상 동작 예상)
- 사용자의 `llama-server.exe` (llama.cpp 공식 release 또는 직접 빌드)
- GGUF 모델
- 빌드 시: Node.js 22+, pnpm 10+, Rust 1.80+, Visual Studio C++ Build Tools

## 빠른 시작

1. Releases에서 `Perch_x.y.z_x64-setup.exe`를 내려받아 설치합니다.
2. 첫 실행 마법사에서 `llama-server.exe`(또는 llama.cpp 압축 해제 폴더)와 GGUF 모델을 선택합니다.
3. **실행 파라미터**에서 포트와 GPU 레이어를 확인한 뒤 **시작**을 클릭합니다.
4. **실행 중**이 되면 **Web UI 열기**로 `http://127.0.0.1:<포트>`에 접속합니다.

> 서명되지 않은 빌드이므로 첫 실행 시 SmartScreen 경고가 표시될 수 있습니다. **추가 정보 → 실행**을 선택하세요. 이 저장소의 Releases에서만 내려받으세요.

## 사용법

### 실행 파라미터

주요 항목은 폼으로 편집합니다. **고급 파라미터**는 `llama-server --help`에서 동적으로 생성되며 검색과 즐겨찾기를 지원합니다. 명령줄 미리보기는 중복·덮어쓰기를 경고합니다. `.bat`으로 내보내면 Perch 없이 실행할 수 있습니다.

### 프로필

프로필 하나가 모델 하나와 설정 한 세트에 대응합니다. 이름이 같은 가져오기는 자동으로 이름이 변경됩니다. 마스킹 내보내기는 모델 경로를 자리표시자로 바꿉니다.

### 모니터링과 진단

대시보드는 기본 1초 간격, 5분 창으로 샘플링합니다. GPU 정보는 NVML을 우선 사용하고 실패하면 `nvidia-smi`로 대체합니다. 멀티 GPU 환경에서는 설정에서 모니터링할 GPU를 선택할 수 있습니다. NVIDIA GPU가 없으면 CPU 모드로 동작합니다. 실행 실패 시 진단 결과, 전체 명령줄, 로그 마지막 부분을 보여줍니다.

### 트레이와 종료

창을 닫으면 기본적으로 트레이로 숨겨집니다(설정에서 변경 가능). 종료 시 llama-server를 유지할지 선택할 수 있습니다.

## CLI

```powershell
perch.exe --cli status
perch.exe --cli start [--profile <이름>]
perch.exe --cli stop
perch.exe --cli export <디렉터리>
```

`status/start/stop`은 명명된 파이프로 실행 중인 GUI와 통신합니다. `start`는 GUI가 없으면 실행을 시도합니다. 출력은 JSON입니다. `export`는 오프라인으로 설정, 도움말 캐시, 로그를 내보냅니다.

## 로컬 읽기 전용 API

설정에서 활성화하고 포트를 지정합니다(기본 `3999`, `127.0.0.1`만 바인딩). 토큰이 설정된 경우 다음 헤더를 추가합니다.

```http
Authorization: Bearer <token>
```

```powershell
curl.exe http://127.0.0.1:3999/status -H "Authorization: Bearer <token>"
curl.exe http://127.0.0.1:3999/metrics -H "Authorization: Bearer <token>"
curl.exe http://127.0.0.1:3999/profiles -H "Authorization: Bearer <token>"
```

`/profiles`의 환경 변수 값은 마스킹됩니다.

## 데이터와 개인정보

- 설정: `%USERPROFILE%\.perch\config.toml`
- 도움말 캐시: `%USERPROFILE%\.perch\help_cache.json`
- 로그: `%USERPROFILE%\.perch\logs\server_<timestamp>.log`
- 모든 데이터는 로컬에만 저장됩니다. Perch는 텔레메트리나 클라우드 요청을 보내지 않습니다.

## 소스에서 빌드

```powershell
git clone https://github.com/Jack-Sai/perch.git
cd perch
pnpm install
pnpm tauri build
```

개발:

```powershell
pnpm tauri dev
```

검증:

```powershell
pnpm build
cd src-tauri
cargo test
cargo clippy
```

## 서드파티 의존성과 라이선스

주요 의존성(전체 목록은 `src-tauri/Cargo.lock`과 `pnpm-lock.yaml`):

| 의존성 | 용도 | 라이선스 |
| --- | --- | --- |
| Tauri 2 / tauri-plugin-* | 데스크톱 셸, 대화상자, 알림, 자동 시작, 링크 열기 | MIT / Apache-2.0 |
| axum / tokio | 로컬 읽기 전용 API | MIT |
| serde / serde_json / toml | 설정과 직렬화 | MIT / Apache-2.0 |
| sysinfo | 프로세스·메모리 모니터링 | MIT |
| nvml-wrapper | NVML 기반 GPU 지표 | MIT / Apache-2.0 |
| windows-sys | Win32 API(명명된 파이프, 뮤텍스, 콘솔) | MIT / Apache-2.0 |
| React / Vite | 프런트엔드 | MIT |
| Tailwind CSS | 스타일 | MIT |
| Recharts | 차트 | MIT |
| lucide-react | 아이콘 | ISC |

GPU 정보는 NVML(존재하는 경우 시스템 `nvml.dll`을 런타임 로드)을 우선 사용하고, 실패하면 `nvidia-smi`로 대체합니다. Perch는 NVML을 정적 링크하거나 NVIDIA 구성 요소를 배포하지 않습니다. llama.cpp와 모델은 포함되지 않으며 각 라이선스를 준수해야 합니다.

## 기여

[CONTRIBUTING.md](CONTRIBUTING.md)와 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)를 참고하세요. 변경 내역은 [CHANGELOG.md](CHANGELOG.md)에 기록됩니다.

## 라이선스

[MIT](LICENSE) © 2026 Jack-Sai
