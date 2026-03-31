# Pokemon Card Price Tracker — Backend Pre-Plan v1

> 작성일: 2026-03-31
> 상태: 초안 (Draft)
> 저장소: `pokemon-price-tracker-backend`

---

## 1. 프로젝트 개요

포켓몬 트레이딩 카드게임 라이브(PTCGL) 카드의 국내 시세를 한눈에 비교할 수 있는 웹 서비스의 **백엔드 API 서버** 프로젝트.
카드 메타데이터 조회와 실시간 시세 조회 API를 제공한다.

전체 시스템은 3개 저장소로 구성된다:

| 저장소 | 역할 |
|--------|------|
| `pokemon-price-tracker-front` | React SPA — UI, 라우팅, API 연동 |
| **`pokemon-price-tracker-backend`** (본 저장소) | Rust Axum API 서버 — 카드 메타데이터 조회 + 실시간 시세 조회 |
| `pokemon-price-tracker-scraper` | Python 스크래퍼 — 공식 카드 DB 수집 + 판매처별 시세 수집 |

---

## 2. 기술 스택

| 항목 | 선택 |
|------|------|
| 언어 | Rust |
| 웹 프레임워크 | Axum 0.8 |
| ORM | SeaORM 1.x |
| DB | SQLite (WAL 모드) |
| 비동기 런타임 | Tokio |
| HTTP 클라이언트 | reqwest |
| CORS | tower-http |
| 직렬화 | serde / serde_json |

---

## 3. API 엔드포인트 설계

| Method | Path | 설명 |
|--------|------|------|
| `GET` | `/api/cards` | 카드 목록 (페이지네이션, 검색·필터 지원) |
| `GET` | `/api/cards/:id` | 카드 단건 조회 |
| `GET` | `/api/cards/:id/prices` | 카드 실시간 시세 조회 (모든 판매처) |
| `GET` | `/api/expansions` | 확장팩 목록 |

### Query 파라미터 (`GET /api/cards`)

| 파라미터 | 타입 | 설명 |
|----------|------|------|
| `q` | string | 카드명 검색 |
| `expansion` | string | 확장팩 필터 |
| `rarity` | string | 희귀도 필터 |
| `page` | int | 페이지 번호 (기본값: 1) |
| `per_page` | int | 페이지당 건수 (기본값: 40) |

---

## 4. 데이터베이스 (SQLite)

> SQLite DB 파일은 `pokemon-price-tracker-scraper`에서 초기 적재한 뒤 본 서버에서 읽기 전용으로 사용한다.

### 스키마

```sql
-- 카드 메타데이터 (공식 DB 기반, scraper가 적재)
CREATE TABLE cards (
  id          TEXT PRIMARY KEY,   -- 카드 번호 (예: SV1a-066)
  name        TEXT NOT NULL,      -- 카드명 (한글)
  expansion   TEXT NOT NULL,      -- 확장팩명
  rarity      TEXT,               -- 희귀도 (C, U, R, RR, SAR 등)
  card_type   TEXT,               -- 포켓몬 / 트레이너 / 에너지
  image_url   TEXT,               -- 공식 이미지 URL
  official_url TEXT,              -- 공식 사이트 상세 URL
  created_at  TEXT DEFAULT (datetime('now')),
  updated_at  TEXT DEFAULT (datetime('now'))
);

-- 카드명 별칭 (판매처별 표기 정규화, scraper가 적재)
CREATE TABLE card_aliases (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  card_id     TEXT NOT NULL REFERENCES cards(id),
  alias       TEXT NOT NULL,      -- 판매처 표기 원문
  source      TEXT NOT NULL,      -- 출처 (cardnyang, icu, daangn 등)
  UNIQUE(alias, source)
);
```

### SeaORM 엔티티

`sea-orm-cli generate entity`로 자동 생성 후 `src/entities/`에 배치.

---

## 5. 실시간 시세 조회 구현

`GET /api/cards/:id/prices` 처리 흐름:

```
1. Axum 핸들러 수신
2. 스크래퍼 서비스 호출 (Python scraper를 subprocess 또는 HTTP로 호출)
3. 4개 판매처 병렬 fetch (tokio::join!)
4. 타임아웃 설정: 판매처별 최대 5초
5. 성공한 판매처 결과만 취합해 JSON 반환
```

> **스크래퍼 호출 방식**: 초기에는 subprocess 방식으로 단순하게 시작. 추후 부하 증가 시 별도 서비스로 분리 고려.

### 시세 비교 대상 판매처

> 판매처별 스크래퍼 상세는 `pokemon-price-tracker-scraper` 저장소의 문서를 참조한다.

| 판매처 | 데이터 종류 | 비고 |
|--------|------------|------|
| 카드냥 (역삼) | 매입가 | 정적 HTML, 빠른 응답 |
| ICU (너정다) | 평균 거래가, 거래 내역 | JS 렌더링, Playwright 필요 |
| 당근마켓 | 중고 판매가 | JSON-LD 파싱 |
| 중고나라 | 중고 판매가 | Next.js SSR JSON 파싱 |

> **시세 데이터는 DB에 저장하지 않는다.** 요청 시마다 실시간 스크래핑 후 즉시 반환.

---

## 6. CORS 설정

Cloudflare Pages 도메인에서 API 호출 허용:

```rust
let cors = CorsLayer::new()
    .allow_origin(["https://your-app.pages.dev".parse().unwrap()])
    .allow_methods([Method::GET]);
```

---

## 7. 아키텍처 다이어그램 (백엔드 관점)

```
[프론트엔드 — Cloudflare Pages]
    |
    | REST API (HTTPS)
    v
[Rust Axum 서버] (본 저장소)
    |
    |-- [카드 메타데이터 조회]
    |       SeaORM → SQLite (공식 DB 데이터, scraper가 사전 적재)
    |
    |-- [실시간 시세 조회]
    |       요청 시마다 Python 스크래퍼 호출 → 판매처별 가격 수집 후 즉시 반환
    |       |-- cardnyang.com   → 매입가
    |       |-- icu.gg          → 평균 거래가
    |       |-- daangn.com      → 중고 판매가
    |       |-- joongna.com     → 중고 판매가
    |
    v
[SQLite DB]  ← 카드 메타데이터만 저장 (시세 데이터 비저장)
```

---

## 8. 주요 고려사항 및 리스크

| 항목 | 내용 |
|------|------|
| SQLite 동시성 | 스크래퍼(쓰기)와 API 서버(읽기)가 동시 접근 시 WAL 모드 활성화 필요 |
| 스크래퍼 호출 방식 | subprocess vs HTTP 마이크로서비스 — 초기엔 subprocess, 부하 증가 시 분리 |
| 실시간 시세 응답 지연 | 판매처별 타임아웃 5초 설정, 부분 실패 허용 (성공한 결과만 반환) |
| 배포 환경 | Rust 바이너리 + SQLite 파일 → Docker 또는 VPS 배포 |

---

## 9. 개발 단계 (백엔드 로드맵)

| 단계 | 내용 |
|------|------|
| Phase 1 | Rust Axum 프로젝트 초기화 + SeaORM 엔티티 생성 |
| Phase 2 | `/api/cards`, `/api/cards/:id`, `/api/expansions` 엔드포인트 구현 |
| Phase 3 | `/api/cards/:id/prices` 실시간 시세 조회 구현 (스크래퍼 연동) |
| Phase 4 | CORS 설정 + 프론트엔드 연동 테스트 |
| Phase 5 | 성능 개선 (in-memory 캐시 TTL 30초, 병렬 처리 최적화) |

---

## 10. 미결 사항 (TBD)

- [ ] 스크래퍼 호출 방식 최종 결정 (subprocess vs HTTP 서비스)
- [ ] 실시간 시세 조회 응답 지연 대응 전략 (타임아웃, 부분 실패 처리)
- [ ] Cloudflare Workers vs. 별도 Rust 서버 호스팅 방식 최종 결정
- [ ] Playwright 서버 운영 방식 (Docker 컨테이너 별도 구성 여부)
- [ ] API 응답 캐싱 전략 (TTL, 캐시 무효화)

### 확정된 사항 (미결에서 해소)

| 항목 | 결정 |
|------|------|
| 시세 히스토리 | **미보존** — 실시간 스크래핑으로 즉시 반환 |
| 로그인/회원 기능 | **없음** (현재 범위 밖) |
| DB | SQLite (파일 기반, scraper와 공유) |
