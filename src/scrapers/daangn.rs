use crate::models::price::PriceItem;
use anyhow::Result;

pub async fn fetch_listings(card_name: &str) -> Result<Vec<PriceItem>> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; PokemonPriceTracker/1.0)")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let encoded_name = urlencoding::encode(card_name).into_owned();
    let url = format!("https://www.daangn.com/search/{}", encoded_name);
    let fetched_at = chrono::Utc::now().to_rfc3339();

    let html = client
        .get(&url)
        .send()
        .await?
        .text()
        .await?;

    let mut results: Vec<PriceItem> = Vec::new();

    // Extract JSON-LD blocks from the HTML
    let mut search_start = 0;
    while let Some(start) = html[search_start..].find(r#"<script type="application/ld+json">"#) {
        let abs_start = search_start + start;
        let content_start = abs_start + r#"<script type="application/ld+json">"#.len();
        if let Some(end_rel) = html[content_start..].find("</script>") {
            let content = &html[content_start..content_start + end_rel];
            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(content) {
                parse_ld_json(&json_val, card_name, &url, &fetched_at, &mut results);
            }
            search_start = content_start + end_rel + "</script>".len();
        } else {
            break;
        }
    }

    Ok(results)
}

fn parse_ld_json(
    val: &serde_json::Value,
    card_name: &str,
    source_url: &str,
    fetched_at: &str,
    results: &mut Vec<PriceItem>,
) {
    // Handle @graph array or individual items
    if let Some(graph) = val.get("@graph").and_then(|v| v.as_array()) {
        for item in graph {
            try_extract_item(item, card_name, source_url, fetched_at, results);
        }
    } else if let Some(items) = val.as_array() {
        for item in items {
            try_extract_item(item, card_name, source_url, fetched_at, results);
        }
    } else {
        try_extract_item(val, card_name, source_url, fetched_at, results);
    }
}

fn try_extract_item(
    item: &serde_json::Value,
    card_name: &str,
    source_url: &str,
    fetched_at: &str,
    results: &mut Vec<PriceItem>,
) {
    let item_type = item
        .get("@type")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Look for Product or ItemList entries
    if item_type == "Product" || item_type == "ListItem" {
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Only include items that mention the card name
        if name.is_empty()
            || !name
                .to_lowercase()
                .contains(&card_name.to_lowercase())
        {
            return;
        }

        // Try to extract price from offers
        let price_opt = item
            .get("offers")
            .and_then(|o| o.get("price").or_else(|| o.get("lowPrice")))
            .and_then(|p| p.as_str().or_else(|| p.as_f64().map(|_| "")).map(|s| s.to_string()))
            .or_else(|| {
                item.get("offers")
                    .and_then(|o| o.get("price"))
                    .and_then(|p| p.as_f64())
                    .map(|f| f.to_string())
            });

        let price = if let Some(price_str) = price_opt {
            let cleaned = price_str.replace([',', '.'], "");
            cleaned.parse::<i64>().unwrap_or(0)
        } else {
            0
        };

        if price <= 0 {
            return;
        }

        let item_url = item
            .get("url")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| Some(source_url.to_string()));

        results.push(PriceItem {
            source: "daangn".to_string(),
            source_name: "당근마켓".to_string(),
            card_id: None,
            card_name_raw: name.to_string(),
            price,
            price_type: "used".to_string(),
            url: item_url,
            fetched_at: fetched_at.to_string(),
        });
    }
}
