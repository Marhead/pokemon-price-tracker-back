use axum::{
    extract::{Path, State},
    response::Json,
};
use sea_orm::EntityTrait;
use std::time::Instant;
use tokio::time::{timeout, Duration};

use crate::{
    entities::cards::Entity as CardEntity,
    error::AppError,
    models::price::PricesResponse,
    scrapers::{cardnyang, daangn, joongna},
    AppState,
};

pub async fn get_prices(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PricesResponse>, AppError> {
    if let Some(cached) = state.price_cache.get(&id).await {
        tracing::info!(card_id = %id, "price cache hit");
        return Ok(Json(cached));
    }

    let card = CardEntity::find_by_id(&id)
        .one(&*state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    let card_name = card.name.clone();
    let card_id_ref: &str = &id;

    let scraper_timeout = Duration::from_secs(10);
    let total_start = Instant::now();

    let (cardnyang_result, daangn_result, joongna_result) = tokio::join!(
        timeout(scraper_timeout, cardnyang::fetch_prices(Some(card_id_ref))),
        timeout(scraper_timeout, daangn::fetch_listings(&card_name)),
        timeout(scraper_timeout, joongna::fetch_listings(&card_name)),
    );

    let mut all_prices = Vec::new();
    let mut errors = Vec::new();

    match cardnyang_result {
        Ok(Ok(prices)) => {
            tracing::info!(card_id = %id, source = "cardnyang", count = prices.len(), "scraper success");
            all_prices.extend(prices);
        }
        Ok(Err(e)) => {
            tracing::warn!(card_id = %id, source = "cardnyang", error = %e, "scraper error");
            errors.push(format!("cardnyang: {}", e));
        }
        Err(_) => {
            tracing::warn!(card_id = %id, source = "cardnyang", "scraper timeout");
            errors.push("cardnyang: timeout".to_string());
        }
    }

    match daangn_result {
        Ok(Ok(prices)) => {
            tracing::info!(card_id = %id, source = "daangn", count = prices.len(), "scraper success");
            all_prices.extend(prices);
        }
        Ok(Err(e)) => {
            tracing::warn!(card_id = %id, source = "daangn", error = %e, "scraper error");
            errors.push(format!("daangn: {}", e));
        }
        Err(_) => {
            tracing::warn!(card_id = %id, source = "daangn", "scraper timeout");
            errors.push("daangn: timeout".to_string());
        }
    }

    match joongna_result {
        Ok(Ok(prices)) => {
            tracing::info!(card_id = %id, source = "joongna", count = prices.len(), "scraper success");
            all_prices.extend(prices);
        }
        Ok(Err(e)) => {
            tracing::warn!(card_id = %id, source = "joongna", error = %e, "scraper error");
            errors.push(format!("joongna: {}", e));
        }
        Err(_) => {
            tracing::warn!(card_id = %id, source = "joongna", "scraper timeout");
            errors.push("joongna: timeout".to_string());
        }
    }

    let elapsed_ms = total_start.elapsed().as_millis();
    tracing::info!(card_id = %id, elapsed_ms = %elapsed_ms, price_count = all_prices.len(), "prices fetched");

    let fetched_at = chrono::Utc::now().to_rfc3339();

    let response = PricesResponse {
        card_id: id.clone(),
        prices: all_prices,
        errors,
        fetched_at,
    };

    state.price_cache.insert(id, response.clone()).await;

    Ok(Json(response))
}
