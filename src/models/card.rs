use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CardResponse {
    pub id: String,
    pub name: String,
    pub expansion: String,
    pub rarity: Option<String>,
    pub card_type: Option<String>,
    pub image_url: Option<String>,
    pub official_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CardListResponse {
    pub cards: Vec<CardResponse>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Deserialize)]
pub struct CardSearchQuery {
    pub q: Option<String>,
    pub expansion: Option<String>,
    pub rarity: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}
