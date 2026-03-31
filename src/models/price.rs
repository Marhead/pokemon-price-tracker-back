use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceItem {
    pub source: String,
    pub source_name: String,
    pub card_id: Option<String>,
    pub card_name_raw: String,
    pub price: i64,
    pub price_type: String, // "buy" | "sell" | "used"
    pub url: Option<String>,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricesResponse {
    pub card_id: String,
    pub prices: Vec<PriceItem>,
    pub errors: Vec<String>,
    pub fetched_at: String,
}
