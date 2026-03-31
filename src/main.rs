mod config;
mod db;
mod entities;
mod error;
mod models;
mod routes;
mod scrapers;

use axum::{http::HeaderValue, routing::get, Router};
use moka::future::Cache;
use std::{sync::Arc, time::Duration};
use tower_http::cors::{Any, CorsLayer};

use config::Config;
use models::price::PricesResponse;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<sea_orm::DatabaseConnection>,
    pub price_cache: Cache<String, PricesResponse>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env();

    let db = db::connect(&config.database_url)
        .await
        .expect("DB 연결 실패");

    let price_cache = Cache::builder()
        .time_to_live(Duration::from_secs(30))
        .max_capacity(1000)
        .build();

    let state = AppState {
        db: Arc::new(db),
        price_cache,
    };

    let cors = build_cors(&config.allowed_origins);

    let app = Router::new()
        .route("/api/cards", get(routes::cards::list_cards))
        .route("/api/cards/{id}", get(routes::cards::get_card))
        .route("/api/cards/{id}/prices", get(routes::prices::get_prices))
        .route("/api/expansions", get(routes::cards::list_expansions))
        .route("/health", get(|| async { "ok" }))
        .layer(cors)
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("서버 시작: http://{}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn build_cors(allowed_origins: &[String]) -> CorsLayer {
    let base = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any);

    let is_wildcard = allowed_origins.is_empty()
        || allowed_origins.iter().any(|o| o.trim() == "*");

    if is_wildcard {
        base.allow_origin(Any)
    } else {
        let origins: Vec<HeaderValue> = allowed_origins
            .iter()
            .filter_map(|o| o.trim().parse::<HeaderValue>().ok())
            .collect();
        base.allow_origin(origins)
    }
}
