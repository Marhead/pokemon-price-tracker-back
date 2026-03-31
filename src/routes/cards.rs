use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use sea_orm::{
    ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};

use crate::{
    entities::cards::{self, Column, Entity as CardEntity},
    error::AppError,
    models::card::{CardListResponse, CardResponse, CardSearchQuery},
    AppState,
};

fn model_to_response(model: cards::Model) -> CardResponse {
    CardResponse {
        id: model.id,
        name: model.name,
        expansion: model.expansion,
        rarity: model.rarity,
        card_type: model.card_type,
        image_url: model.image_url,
        official_url: model.official_url,
    }
}

pub async fn list_cards(
    State(state): State<AppState>,
    Query(params): Query<CardSearchQuery>,
) -> Result<Json<CardListResponse>, AppError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(40).clamp(1, 200);

    let mut query = CardEntity::find();

    if let Some(ref q) = params.q {
        if !q.is_empty() {
            query = query.filter(Column::Name.contains(q.as_str()));
        }
    }
    if let Some(ref expansion) = params.expansion {
        if !expansion.is_empty() {
            query = query.filter(Column::Expansion.eq(expansion.as_str()));
        }
    }
    if let Some(ref rarity) = params.rarity {
        if !rarity.is_empty() {
            query = query.filter(Column::Rarity.eq(rarity.as_str()));
        }
    }

    let total = query.clone().count(&*state.db).await?;

    let offset = ((page - 1) * per_page) as u64;
    let cards = query
        .order_by_asc(Column::Name)
        .offset(offset)
        .limit(per_page as u64)
        .all(&*state.db)
        .await?;

    let card_responses = cards.into_iter().map(model_to_response).collect();

    Ok(Json(CardListResponse {
        cards: card_responses,
        total,
        page,
        per_page,
    }))
}

pub async fn get_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<CardResponse>, AppError> {
    let card = CardEntity::find_by_id(&id)
        .one(&*state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(model_to_response(card)))
}

pub async fn list_expansions(
    State(state): State<AppState>,
) -> Result<Json<Vec<String>>, AppError> {
    let results = CardEntity::find()
        .select_only()
        .column(Column::Expansion)
        .order_by_asc(Column::Expansion)
        .into_tuple::<String>()
        .all(&*state.db)
        .await?;

    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    let expansions: Vec<String> = results
        .into_iter()
        .filter(|e| seen.insert(e.clone()))
        .collect();

    Ok(Json(expansions))
}
