# Pokemon Card Price Tracker — Backend Execute Plan v1

> 작성일: 2026-03-31
> 기반 문서: `pre-plan-v1.md`
> 상태: 실행 계획 (Ready to Execute)
> 저장소: `pokemon-price-tracker-backend`

---

## 실행 순서 개요

```
Phase 1  →  Rust Axum 프로젝트 초기화 + SeaORM 엔티티 생성
Phase 2  →  카드 메타데이터 API 구현
Phase 3  →  실시간 시세 조회 API 구현 (스크래퍼 연동)
Phase 4  →  CORS 설정 + 프론트엔드 연동 테스트
Phase 5  →  성능 개선 및 고도화
```

---

## Phase 1 — Rust Axum 프로젝트 초기화 + SeaORM 엔티티 생성

### 목표
Axum 기반 HTTP 서버 프로젝트를 구성하고, SQLite DB에 맞는 SeaORM 엔티티를 생성한다.

### 작업 목록

#### 1-1. Rust 프로젝트 초기화

```bash
cargo init --bin
```

#### 1-2. Cargo.toml 의존성 추가

```toml
[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
sea-orm = { version = "1", features = ["sqlx-sqlite", "runtime-tokio-rustls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tower-http = { version = "0.6", features = ["cors"] }
reqwest = { version = "0.12", features = ["json"] }
dotenvy = "0.15"
tracing = "0.1"
tracing-subscriber = "0.3"
```

#### 1-3. 기본 서버 뼈대

`src/main.rs`:
```rust
#[tokio::main]
async fn main() {
    tracing_subscriber::init();
    dotenvy::dotenv().ok();

    let db = db::connect().await;
    let app = routes::create_router(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

#### 1-4. DB 연결 모듈

`src/db.rs`:
```rust
use sea_orm::{Database, DatabaseConnection};

pub async fn connect() -> DatabaseConnection {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://db/pokemon_cards.db".to_string());
    Database::connect(&db_url).await.unwrap()
}
```

#### 1-5. SeaORM 엔티티 생성

```bash
sea-orm-cli generate entity \
  -u sqlite://path/to/pokemon_cards.db \
  -o src/entities
```

> SQLite DB 파일은 `pokemon-price-tracker-scraper`에서 사전 적재한 것을 사용한다.

#### 1-6. 프로젝트 디렉토리 구조

```
src/
  main.rs
  config.rs        ← 환경 변수, 설정
  db.rs            ← DB 연결
  error.rs         ← 에러 타입 통합
  entities/
    mod.rs
    cards.rs       ← SeaORM 자동 생성
  models/
    mod.rs
    card.rs        ← API 응답 DTO
    price.rs       ← 시세 응답 DTO
  routes/
    mod.rs         ← 라우터 조립
    cards.rs       ← /api/cards, /api/cards/:id
    prices.rs      ← /api/cards/:id/prices
  scrapers/
    mod.rs
    cardnyang.rs   ← 카드냥 스크래퍼 호출
    daangn.rs      ← 당근마켓 스크래퍼 호출
    joongna.rs     ← 중고나라 스크래퍼 호출
```

### 완료 기준
- `cargo build` 성공
- `cargo run` 후 `http://localhost:3000` 응답 확인
- SeaORM 엔티티 파일 생성 완료 (`src/entities/cards.rs`)

---

## Phase 2 — 카드 메타데이터 API 구현

### 목표
카드 목록 조회, 카드 단건 조회, 확장팩 목록 API를 구현한다.

### 작업 목록

#### 2-1. 응답 DTO 정의

`src/models/card.rs`:
```rust
#[derive(Serialize)]
pub struct CardResponse {
    pub id: String,
    pub name: String,
    pub expansion: String,
    pub rarity: Option<String>,
    pub card_type: Option<String>,
    pub image_url: Option<String>,
    pub official_url: Option<String>,
}

#[derive(Serialize)]
pub struct CardListResponse {
    pub cards: Vec<CardResponse>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
}
```

#### 2-2. GET /api/cards 구현

`src/routes/cards.rs`:

Query 파라미터:
| 파라미터 | 타입 | 설명 |
|----------|------|------|
| `q` | string | 카드명 검색 (LIKE) |
| `expansion` | string | 확장팩 필터 |
| `rarity` | string | 희귀도 필터 |
| `page` | int | 페이지 번호 (기본값: 1) |
| `per_page` | int | 페이지당 건수 (기본값: 40) |

```rust
pub async fn list_cards(
    State(db): State<DatabaseConnection>,
    Query(params): Query<CardSearchParams>,
) -> Json<CardListResponse> {
    // SeaORM 쿼리: 필터링 + 페이지네이션
}
```

#### 2-3. GET /api/cards/:id 구현

```rust
pub async fn get_card(
    State(db): State<DatabaseConnection>,
    Path(id): Path<String>,
) -> Result<Json<CardResponse>, AppError> {
    // cards 테이블에서 id로 단건 조회
}
```

#### 2-4. GET /api/expansions 구현

```rust
pub async fn list_expansions(
    State(db): State<DatabaseConnection>,
) -> Json<Vec<String>> {
    // SELECT DISTINCT expansion FROM cards ORDER BY expansion
}
```

#### 2-5. 라우터 조립

`src/routes/mod.rs`:
```rust
pub fn create_router(db: DatabaseConnection) -> Router {
    Router::new()
        .route("/api/cards", get(cards::list_cards))
        .route("/api/cards/:id", get(cards::get_card))
        .route("/api/expansions", get(cards::list_expansions))
        .with_state(db)
}
```

### 완료 기준
- `GET /api/cards` → 카드 목록 JSON 반환 (페이지네이션 동작)
- `GET /api/cards?q=리자몽` → 검색 결과 필터링
- `GET /api/cards/SV1a-066` → 단건 조회
- `GET /api/expansions` → 확장팩 목록 반환

---

## Phase 3 — 실시간 시세 조회 API 구현 (스크래퍼 연동)

### 목표
`/api/cards/:id/prices` 엔드포인트에서 Python 스크래퍼를 호출해 판매처별 실시간 시세를 반환한다.

### 작업 목록

#### 3-1. 시세 응답 DTO 정의

`src/models/price.rs`:
```rust
#[derive(Serialize)]
pub struct PriceItem {
    pub source: String,          // cardnyang, icu, daangn, joongna
    pub card_id: Option<String>,
    pub card_name_raw: String,
    pub price: i64,
    pub price_type: String,      // buy, sell, used
    pub url: Option<String>,
    pub fetched_at: String,
}

#[derive(Serialize)]
pub struct PriceResponse {
    pub card_id: String,
    pub prices: Vec<PriceItem>,
    pub errors: Vec<String>,     // 실패한 판매처 목록
}
```

#### 3-2. 스크래퍼 호출 모듈

`src/scrapers/mod.rs`:

스크래퍼 호출 방식 (초기: subprocess):
```rust
pub async fn fetch_prices(card_id: &str) -> PriceResponse {
    let results = tokio::join!(
        call_scraper("cardnyang", card_id),
        call_scraper("daangn", card_id),
        call_scraper("joongna", card_id),
        // ICU는 Playwright 필요 → 별도 처리
    );
    // 성공한 결과만 취합, 실패한 판매처는 errors에 기록
}

async fn call_scraper(source: &str, card_id: &str) -> Result<Vec<PriceItem>, String> {
    // tokio::time::timeout(Duration::from_secs(5), ...)
    // subprocess로 Python 스크래퍼 호출
    // JSON stdout 파싱
}
```

> `pokemon-price-tracker-scraper` 저장소의 `prices/runner.py`를 호출한다.

#### 3-3. GET /api/cards/:id/prices 핸들러

`src/routes/prices.rs`:
```rust
pub async fn get_prices(
    Path(id): Path<String>,
) -> Json<PriceResponse> {
    scrapers::fetch_prices(&id).await
}
```

#### 3-4. 라우터에 시세 엔드포인트 추가

```rust
.route("/api/cards/:id/prices", get(prices::get_prices))
```

#### 3-5. 타임아웃 및 부분 실패 처리

- 판매처별 타임아웃: 5초
- 타임아웃·오류 발생 시 해당 판매처만 skip, 나머지 정상 반환
- `PriceResponse.errors`에 실패한 판매처명 기록

### 완료 기준
- `GET /api/cards/SV1a-066/prices` → 실시간 시세 JSON 반환
- 4개 판매처 중 일부 실패해도 나머지 결과 정상 반환
- 각 판매처 5초 이내 응답 (타임아웃 동작 확인)

---

## Phase 4 — CORS 설정 + 프론트엔드 연동 테스트

### 목표
프론트엔드(Cloudflare Pages)에서 API 호출이 가능하도록 CORS를 설정하고, 실제 연동을 테스트한다.

### 작업 목록

#### 4-1. CORS 미들웨어 추가

`src/routes/mod.rs`:
```rust
use tower_http::cors::{CorsLayer, Any};
use http::Method;

pub fn create_router(db: DatabaseConnection) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)  // 개발 시 Any, 운영 시 특정 도메인으로 변경
        .allow_methods([Method::GET])
        .allow_headers(Any);

    Router::new()
        .route("/api/cards", get(cards::list_cards))
        .route("/api/cards/:id", get(cards::get_card))
        .route("/api/cards/:id/prices", get(prices::get_prices))
        .route("/api/expansions", get(cards::list_expansions))
        .layer(cors)
        .with_state(db)
}
```

#### 4-2. 환경별 CORS 설정

`.env`:
```
DATABASE_URL=sqlite://db/pokemon_cards.db
CORS_ORIGIN=https://your-app.pages.dev
```

운영 시 `CORS_ORIGIN` 환경 변수로 허용 도메인 지정.

#### 4-3. 프론트엔드 연동 테스트

- 프론트엔드(`pokemon-price-tracker-front`) 로컬 dev 서버 실행
- 백엔드 로컬 서버 실행 (`cargo run`)
- 프론트엔드에서 카드 목록 API 호출 → 정상 렌더링 확인
- 카드 상세 → 시세 조회 → PriceTable 렌더링 확인
- CORS preflight 요청 정상 처리 확인

### 완료 기준
- 프론트엔드 → 백엔드 API 호출 시 CORS 에러 없음
- 카드 검색 → 상세 → 시세 조회 전체 플로우 정상 동작
- 개발/운영 환경별 CORS 설정 분리

---

## Phase 5 — 성능 개선 및 고도화

### 목표
API 응답 속도 개선 및 운영 안정성을 높인다.

### 작업 목록

#### 5-1. 시세 응답 캐싱

- In-memory 캐시 (TTL 30초)
- 동일 카드 ID에 대해 30초 내 재요청 시 캐시된 결과 반환
- 캐시 키: `card_id`

```rust
// HashMap<String, (Instant, PriceResponse)> 기반 간단한 TTL 캐시
// 또는 moka 크레이트 활용
```

#### 5-2. 스크래퍼 병렬 처리 최적화

- `tokio::join!` 대신 `tokio::select!` + timeout 조합으로 더 빠른 응답
- 가장 먼저 완료된 판매처부터 즉시 포함

#### 5-3. 구조화된 로깅

```rust
tracing::info!(card_id = %id, source = "cardnyang", elapsed_ms = %ms, "price fetched");
tracing::warn!(card_id = %id, source = "icu", error = %e, "scraper timeout");
```

#### 5-4. 헬스체크 엔드포인트

```rust
.route("/health", get(|| async { "ok" }))
```

#### 5-5. Docker 배포 준비

```dockerfile
FROM rust:1.77 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/server /usr/local/bin/
COPY db/pokemon_cards.db /app/db/
CMD ["server"]
```

### 완료 기준
- 동일 카드 연속 시세 조회 시 2번째부터 캐시 응답 (30초 이내)
- 스크래퍼 타임아웃 시 나머지 결과 즉시 반환
- Docker 이미지 빌드 + 실행 정상

---

## 의존성 요약

| 크레이트 | 버전 | 용도 |
|----------|------|------|
| axum | 0.8 | HTTP 서버 |
| tokio | 1 | 비동기 런타임 |
| sea-orm | 1 | ORM (SQLite) |
| serde / serde_json | 1 | JSON 직렬화 |
| tower-http | 0.6 | CORS 미들웨어 |
| reqwest | 0.12 | HTTP 클라이언트 |
| dotenvy | 0.15 | 환경 변수 로딩 |
| tracing | 0.1 | 구조화 로깅 |

---

## 체크리스트

### Phase 1
- [ ] Cargo 프로젝트 초기화 + 의존성 추가
- [ ] 기본 서버 뼈대 (`main.rs`, `db.rs`)
- [ ] SeaORM 엔티티 생성 (`src/entities/`)
- [ ] `cargo build` + `cargo run` 성공

### Phase 2
- [ ] 응답 DTO 정의 (`models/card.rs`)
- [ ] `GET /api/cards` 구현 (검색 + 필터 + 페이지네이션)
- [ ] `GET /api/cards/:id` 구현
- [ ] `GET /api/expansions` 구현
- [ ] API 동작 테스트 (curl 또는 Postman)

### Phase 3
- [ ] 시세 응답 DTO 정의 (`models/price.rs`)
- [ ] 스크래퍼 호출 모듈 (`scrapers/`)
- [ ] `GET /api/cards/:id/prices` 구현
- [ ] 타임아웃 + 부분 실패 처리 확인
- [ ] 4개 판매처 연동 테스트

### Phase 4
- [ ] CORS 미들웨어 설정
- [ ] 환경별 CORS origin 분리
- [ ] 프론트엔드 연동 전체 플로우 테스트

### Phase 5
- [ ] In-memory 시세 캐시 (TTL 30초)
- [ ] 스크래퍼 병렬 처리 최적화
- [ ] 구조화 로깅 추가
- [ ] 헬스체크 엔드포인트
- [ ] Dockerfile 작성 + 빌드 테스트
