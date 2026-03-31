# 도커화 하기

> 작성일: 2026-03-31
> 대상: `pokemon-price-tracker-backend`

---

## 현재 상태

- Rust edition: **2021** (변경 필요)
- 기존 Dockerfile 존재하나 KST/로그 설정 미흡
- 패키지 버전이 범위 지정(`"0.8"`, `"1"` 등)으로 느슨하게 관리됨
- `.dockerignore` 없음
- `.env` 파일로 환경변수를 로컬에서 관리 중 (`dotenvy` 크레이트 사용)
- `docker-compose.yml` 없음

---

## 요구사항

### 1. Multistage 빌드하기

- Builder 스테이지: 의존성 캐싱 레이어 분리 (Cargo.toml/Cargo.lock만 먼저 복사 → 빌드 → 소스 복사 → 재빌드)
- Runtime 스테이지: `debian:bookworm-slim` 기반 최소 이미지
- 빌드 아티팩트(`target/release/server`)만 런타임으로 복사

### 2. 한국 시간 설정 필수

- 컨테이너 내 타임존을 `Asia/Seoul` (KST, UTC+9)로 설정
- `tzdata` 패키지 설치 + `TZ` 환경변수 + `/etc/localtime` 심링크
- chrono / tracing 로그 타임스탬프에 KST가 반영되어야 함

### 3. 로그 출력도 신경 쓰기

- `RUST_LOG` 환경변수로 로그 레벨 제어 가능하게 (기본값: `info`)
- tracing-subscriber에 `env-filter` 기능 활용 (이미 feature 활성화됨)
- `main.rs`에서 `fmt::init()` → `EnvFilter` 기반 초기화로 변경
- Docker에서 `RUST_LOG=info` 기본 설정

### 4. 빌드는 자유롭게 하고 배포 버전을 빌드 된 아티팩트로 하기

- Builder 스테이지에서 `cargo build --release`로 최적화 빌드
- Runtime에는 컴파일된 바이너리 + 필수 런타임 라이브러리(ca-certificates, tzdata)만 포함
- Rust 툴체인, 소스코드, target 디렉토리는 런타임에 포함하지 않음

### 5. Rust edition은 반드시 2024 혹은 그 이상

- `Cargo.toml`의 `edition`을 `"2021"` → `"2024"`로 변경
- Docker base 이미지를 `rust:1.94` 이상으로 설정 (edition 2024 지원)
- 로컬 빌드 환경도 `rustc 1.85+` 필요 (현재 로컬: 1.92.0, 충분)

### 6. 패키지들도 일단 먼저 최신화 한번하고 현재기준 제일 최신 버전으로 고정하기

현재 → 최신 매핑:

| 크레이트 | 현재 | 최신 (2026-03 기준) | 비고 |
|----------|------|---------------------|------|
| axum | 0.8 | **0.8.8** | 마이너 패치 |
| tokio | 1 | **1.50.0** | semver 호환 |
| sea-orm | 1 | **1.1.19** | 2.0은 RC, 안정판 유지 |
| serde | 1 | **1.0.228** | semver 호환 |
| serde_json | 1 | **1.0.149** | semver 호환 |
| tower-http | 0.6 | **0.6.8** | 마이너 패치 |
| reqwest | 0.12 | **0.13.2** | 주요 변경: rustls 기본 TLS |
| tracing | 0.1 | **0.1.44** | 패치 |
| tracing-subscriber | 0.3 | **0.3.23** | 패치 |
| dotenvy | 0.15 | — | **삭제 대상** (요구사항 7) |
| thiserror | 1 | **2.0.18** | 메이저 업그레이드 |
| anyhow | 1 | **1.0.102** | 패치 |
| tokio-stream | 0.1 | **0.1.18** | 패치 |
| chrono | 0.4 | **0.4.44** | 패치 |
| regex | 1 | **1.12.3** | 패치 |
| urlencoding | 2 | **2.1.3** | 패치 |
| moka | 0.12 | **0.12.15** | 패치 |

- `reqwest` 0.12 → 0.13: rustls가 기본 TLS로 변경됨. native-tls 시스템 라이브러리 불필요해질 수 있음. `.send()`, `.text()` 등 기본 API는 호환.
- `thiserror` 1 → 2: `#[derive(Error)]` 매크로 API 호환. 내부 구현 개선.

### 7. .env 파일 없이 docker-compose로 환경변수 주입

- 프로젝트에 `.env` 파일을 남기지 않는다
- 기존 `.env` 파일 삭제, `.gitignore`에 `.env` 추가하여 실수 방지
- `dotenvy` 크레이트 의존성 제거 (`config.rs`에서 `dotenvy::dotenv().ok()` 호출도 제거)
- 모든 환경변수(`DATABASE_URL`, `HOST`, `PORT`, `ALLOWED_ORIGINS`, `RUST_LOG`)는 `docker-compose.yml`의 `environment` 섹션에서 주입
- `config.rs`는 `std::env::var()`로만 환경변수를 읽도록 유지 (이미 그렇게 되어 있고, dotenvy 부분만 제거)
- 로컬 개발 시에도 `docker compose up`으로 실행하거나 쉘에서 직접 `export`
