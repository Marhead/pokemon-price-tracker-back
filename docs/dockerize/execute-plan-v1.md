# 도커화 — Execute Plan v1

> 작성일: 2026-03-31
> 기반 문서: `pre-plan-v1.md`
> 상태: 실행 계획 (Ready to Execute)
> 저장소: `pokemon-price-tracker-backend`

---

## 실행 순서 개요

```
Step 1  →  Rust edition 2024 업그레이드
Step 2  →  패키지 최신화 + 버전 고정 (dotenvy 제거 포함)
Step 3  →  .env 제거 + config.rs에서 dotenvy 제거
Step 4  →  로그 초기화 개선 (EnvFilter 적용)
Step 5  →  Dockerfile 재작성 (Multistage + KST + 로그)
Step 6  →  docker-compose.yml 작성
Step 7  →  .dockerignore + .gitignore 정리
Step 8  →  빌드 검증
```

---

## Step 1 — Rust edition 2024 업그레이드

### 목표
`Cargo.toml`의 edition을 `2021` → `2024`로 변경한다.

### 작업

`Cargo.toml`:
```toml
[package]
name = "pokemon-price-tracker-backend"
version = "0.1.0"
edition = "2024"
```

### 주의사항
- edition 2024는 Rust 1.85.0부터 지원
- 로컬 환경(1.92.0) 및 Docker base(1.94)에서 모두 호환
- edition 2024 변경 사항: `unsafe_op_in_unsafe_fn` 기본 경고, `gen` 키워드 예약 등 — 현재 코드에는 영향 없음

### 완료 기준
- `cargo build` 성공 (edition 2024)

---

## Step 2 — 패키지 최신화 + 버전 고정

### 목표
모든 의존성을 2026-03 기준 최신 안정 버전으로 업그레이드하고, 정확한 버전으로 고정한다. `dotenvy`는 제거한다.

### 작업

`Cargo.toml` 의존성을 아래로 교체:

```toml
[dependencies]
axum = { version = "=0.8.8", features = ["macros"] }
tokio = { version = "=1.50.0", features = ["full"] }
sea-orm = { version = "=1.1.19", features = ["sqlx-sqlite", "runtime-tokio-rustls", "macros"] }
serde = { version = "=1.0.228", features = ["derive"] }
serde_json = "=1.0.149"
tower-http = { version = "=0.6.8", features = ["cors", "trace"] }
reqwest = { version = "=0.13.2", features = ["json"] }
tracing = "=0.1.44"
tracing-subscriber = { version = "=0.3.23", features = ["env-filter"] }
thiserror = "=2.0.18"
anyhow = "=1.0.102"
tokio-stream = "=0.1.18"
chrono = { version = "=0.4.44", features = ["serde"] }
regex = "=1.12.3"
urlencoding = "=2.1.3"
moka = { version = "=0.12.15", features = ["future"] }

[dev-dependencies]
tower = { version = "=0.5.3", features = ["util"] }
```

> `dotenvy = "0.15"` 삭제됨 — 요구사항 7에 따라 `.env` 파일을 사용하지 않으므로 불필요.

### reqwest 0.12 → 0.13 마이그레이션

현재 스크래퍼 코드에서 사용하는 API:
- `reqwest::Client::builder()` → 호환
- `.user_agent()`, `.timeout()`, `.build()` → 호환
- `.get().send().await` → 호환
- `.text().await` → 호환

주요 변경점:
- rustls가 기본 TLS 백엔드 (native-tls 시스템 라이브러리 불필요)
- `json` feature를 명시적으로 활성화해야 함 (이미 설정되어 있음)

**코드 변경 불필요** — feature flag(`json`)가 이미 있고, TLS 관련 메서드를 직접 사용하지 않음.

### thiserror 1 → 2 마이그레이션

`src/error.rs`에서 사용하는 API:
- `#[derive(thiserror::Error)]` → 호환
- `#[error("...")]` 속성 → 호환
- `#[from]` 속성 → 호환

**코드 변경 불필요** — derive 매크로 인터페이스 동일.

### 완료 기준
- `cargo update` 후 `cargo build` 성공
- `Cargo.lock`에 고정된 버전 반영 확인
- `dotenvy`가 `Cargo.toml`과 `Cargo.lock`에서 제거됨

---

## Step 3 — .env 제거 + config.rs에서 dotenvy 제거

### 목표
`.env` 파일 기반 환경변수 로딩을 제거한다. 환경변수는 docker-compose 또는 쉘에서 직접 주입한다.

### 작업

#### 3-1. `.env` 파일 삭제

```bash
rm .env
```

현재 `.env` 내용 (참고용, docker-compose.yml에 옮길 값):
```
DATABASE_URL=sqlite:../scraper/db/pokemon_cards.db
HOST=0.0.0.0
PORT=3000
ALLOWED_ORIGINS=http://localhost:5173
```

#### 3-2. `config.rs`에서 dotenvy 호출 제거

현재:
```rust
impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();   // ← 이 줄 삭제
        Self {
            // ...
        }
    }
}
```

변경 후:
```rust
impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:./db/pokemon_cards.db".to_string()),
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .unwrap_or(3000),
            allowed_origins: std::env::var("ALLOWED_ORIGINS")
                .unwrap_or_else(|_| "*".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        }
    }
}
```

변경 포인트:
- `dotenvy::dotenv().ok()` 호출 제거
- `DATABASE_URL` 기본값을 `sqlite:./db/pokemon_cards.db`로 변경 (컨테이너 내 경로 기준)
- `ALLOWED_ORIGINS` 기본값을 `*`로 변경 (설정 누락 시 개발 편의를 위해 전체 허용, 운영은 docker-compose에서 명시)

### 완료 기준
- `.env` 파일이 프로젝트에 존재하지 않음
- `config.rs`에 `dotenvy` 관련 코드 없음
- `cargo build` 성공

---

## Step 4 — 로그 초기화 개선 (EnvFilter 적용)

### 목표
`RUST_LOG` 환경변수로 로그 레벨을 제어할 수 있도록 tracing-subscriber 초기화를 변경한다.

### 현재 코드

`src/main.rs`:
```rust
tracing_subscriber::fmt::init();
```

이 방식은 `RUST_LOG` 환경변수를 무시한다.

### 변경 코드

`src/main.rs`:
```rust
use tracing_subscriber::EnvFilter;

tracing_subscriber::fmt()
    .with_env_filter(
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"))
    )
    .init();
```

- `RUST_LOG` 미설정 시 기본값 `info`
- 컨테이너에서 `RUST_LOG=debug` 등으로 런타임 조정 가능
- `RUST_LOG=pokemon_price_tracker_backend=debug,tower_http=info` 같은 세밀한 제어 가능

### 완료 기준
- `RUST_LOG=debug cargo run` 시 debug 로그 출력
- `RUST_LOG` 미설정 시 info 레벨만 출력

---

## Step 5 — Dockerfile 재작성

### 목표
Multistage 빌드, KST 타임존, 로그 설정이 반영된 프로덕션용 Dockerfile을 작성한다.

### Dockerfile

```dockerfile
# ── Builder ──
FROM rust:1.94-bookworm AS builder
WORKDIR /app

# 의존성 캐싱 레이어
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

# 소스 빌드
COPY src ./src
RUN touch src/main.rs && cargo build --release

# ── Runtime ──
FROM debian:bookworm-slim

# 필수 런타임 패키지 + 한국 시간대
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tzdata \
    && ln -sf /usr/share/zoneinfo/Asia/Seoul /etc/localtime \
    && echo "Asia/Seoul" > /etc/timezone \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/server /usr/local/bin/

ENV TZ=Asia/Seoul
ENV HOST=0.0.0.0
ENV PORT=3000
ENV RUST_LOG=info

EXPOSE 3000
CMD ["server"]
```

### 설계 포인트

| 항목 | 설명 |
|------|------|
| **Multistage** | Builder(~2GB 컴파일 환경) → Runtime(~80MB slim 이미지) |
| **의존성 캐싱** | Cargo.toml/Lock만 먼저 복사하여 소스 변경 시 의존성 재빌드 방지 |
| **KST** | `tzdata` + `TZ=Asia/Seoul` + `/etc/localtime` 심링크 3중 설정 |
| **로그** | `RUST_LOG=info` 기본값, docker-compose에서 오버라이드 가능 |
| **TLS** | reqwest 0.13은 rustls 기본이므로 native-tls 불필요. `ca-certificates`는 인증서 검증용으로 유지 |
| **환경변수** | Dockerfile에는 불변 기본값만 설정. 가변 설정(`DATABASE_URL`, `ALLOWED_ORIGINS` 등)은 docker-compose에서 주입 |

### 완료 기준
- `docker build` 성공
- `docker run` 후 `/health` 응답 확인
- 컨테이너 내 `date` 명령이 KST 출력
- 로그 타임스탬프가 KST

---

## Step 6 — docker-compose.yml 작성

### 목표
`.env` 파일 없이 모든 환경변수를 `docker-compose.yml`에서 관리한다.

### docker-compose.yml

```yaml
services:
  backend:
    build: .
    ports:
      - "3000:3000"
    environment:
      DATABASE_URL: "sqlite:/app/db/pokemon_cards.db"
      HOST: "0.0.0.0"
      PORT: "3000"
      ALLOWED_ORIGINS: "http://localhost:5173"
      RUST_LOG: "info"
      TZ: "Asia/Seoul"
    volumes:
      - ../scraper/db/pokemon_cards.db:/app/db/pokemon_cards.db:ro
    restart: unless-stopped
```

### 환경변수 설명

| 변수 | 설명 | 기본값 (Dockerfile/config.rs) |
|------|------|------|
| `DATABASE_URL` | SQLite DB 경로 | `sqlite:./db/pokemon_cards.db` |
| `HOST` | 바인드 주소 | `0.0.0.0` |
| `PORT` | 서버 포트 | `3000` |
| `ALLOWED_ORIGINS` | CORS 허용 도메인 (콤마 구분) | `*` |
| `RUST_LOG` | 로그 레벨 | `info` |
| `TZ` | 타임존 | `Asia/Seoul` |

### 운영 환경 오버라이드 예시

`docker-compose.prod.yml`:
```yaml
services:
  backend:
    environment:
      ALLOWED_ORIGINS: "https://your-app.pages.dev"
      RUST_LOG: "warn"
```

```bash
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d
```

### 실행 방법

```bash
# 개발 환경 (빌드 + 실행)
docker compose up --build

# 백그라운드 실행
docker compose up -d --build

# 로그 확인
docker compose logs -f backend

# 종료
docker compose down
```

### 완료 기준
- `docker compose up --build` 성공
- 환경변수가 컨테이너에 정상 주입 확인
- `.env` 파일 없이도 정상 동작

---

## Step 7 — .dockerignore + .gitignore 정리

### 목표
Docker 빌드 컨텍스트에서 불필요한 파일을 제외하고, `.env` 파일이 실수로 커밋되지 않도록 한다.

### .dockerignore

```
target/
.git/
.env
docs/
*.md
.gitignore
docker-compose*.yml
```

### .gitignore 추가 항목

기존 `.gitignore`에 아래 항목을 추가:

```
.env
.env.*
```

### 완료 기준
- `docker build` 시 컨텍스트 크기가 소스 코드 수준으로 축소
- `.env` 파일이 git 추적 대상에서 제외

---

## Step 8 — 빌드 검증

### 로컬 빌드 검증

```bash
# edition 2024 + 최신 패키지로 빌드
cargo build --release

# 환경변수 직접 설정 후 서버 실행
DATABASE_URL=sqlite:../scraper/db/pokemon_cards.db cargo run &
curl http://localhost:3000/health
# → "ok"
```

### Docker Compose 검증

```bash
# 빌드 + 실행
docker compose up --build -d

# 검증
curl http://localhost:3000/health          # → "ok"
docker compose exec backend date           # → KST 시간 확인
docker compose logs backend                # → RUST_LOG=info 레벨 로그 확인

# 환경변수 주입 확인
docker compose exec backend env | grep DATABASE_URL
docker compose exec backend env | grep ALLOWED_ORIGINS

# 정리
docker compose down
```

### 완료 기준
- `cargo build --release` 성공 (edition 2024)
- `docker compose up --build` 성공
- `/health` 응답 `"ok"`
- KST 타임존 동작 확인
- `RUST_LOG` 기반 로그 레벨 제어 확인
- `.env` 파일 없이 docker-compose 환경변수만으로 동작

---

## 체크리스트

### Step 1 — Edition 업그레이드
- [ ] `Cargo.toml` edition `"2021"` → `"2024"` 변경
- [ ] `cargo build` 성공

### Step 2 — 패키지 최신화
- [ ] 모든 의존성 최신 안정 버전으로 고정 (`=x.y.z`)
- [ ] `dotenvy` 의존성 제거
- [ ] reqwest 0.12 → 0.13 업그레이드 (코드 변경 불필요 확인)
- [ ] thiserror 1 → 2 업그레이드 (코드 변경 불필요 확인)
- [ ] `cargo update && cargo build` 성공

### Step 3 — .env 제거 + dotenvy 제거
- [ ] `.env` 파일 삭제
- [ ] `config.rs`에서 `dotenvy::dotenv().ok()` 제거
- [ ] `DATABASE_URL` 기본값을 컨테이너 경로로 변경
- [ ] `ALLOWED_ORIGINS` 기본값을 `*`로 변경
- [ ] `cargo build` 성공

### Step 4 — 로그 개선
- [ ] `tracing_subscriber::fmt::init()` → `EnvFilter` 기반으로 변경
- [ ] `RUST_LOG` 환경변수 동작 확인

### Step 5 — Dockerfile
- [ ] Multistage 빌드 (Builder + Runtime)
- [ ] 의존성 캐싱 레이어 분리
- [ ] KST 타임존 설정 (`tzdata` + `TZ` + `/etc/localtime`)
- [ ] `RUST_LOG=info` 기본값 설정
- [ ] rust:1.94-bookworm 기반
- [ ] `docker build` 성공

### Step 6 — docker-compose.yml
- [ ] 모든 환경변수를 `environment` 섹션에 명시
- [ ] SQLite DB 파일 볼륨 마운트
- [ ] `docker compose up --build` 성공
- [ ] `.env` 파일 없이 정상 동작 확인

### Step 7 — .dockerignore + .gitignore
- [ ] `.dockerignore` 작성 (`target/`, `.git/`, `.env`, `docs/` 등)
- [ ] `.gitignore`에 `.env`, `.env.*` 추가

### Step 8 — 검증
- [ ] 로컬 `cargo build --release` 성공
- [ ] Docker Compose 빌드 + 실행 + 헬스체크 정상
- [ ] KST 시간 출력 확인
- [ ] 로그 레벨 제어 확인
- [ ] 환경변수 주입 확인 (`.env` 파일 없이)
