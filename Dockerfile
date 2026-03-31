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
