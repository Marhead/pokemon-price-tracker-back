use crate::models::price::PriceItem;
use anyhow::Result;
use regex::Regex;

const MODES: &[&str] = &["buying", "buying2"];

pub async fn fetch_prices(card_id: Option<&str>) -> Result<Vec<PriceItem>> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; PokemonPriceTracker/1.0)")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let price_re = Regex::new(r"\d[\d,]*")?;
    let card_code_re = Regex::new(r"[A-Z]{1,3}\d*[a-z]?-\d{3}")?;

    let fetched_at = chrono::Utc::now().to_rfc3339();
    let mut results: Vec<PriceItem> = Vec::new();

    for mode in MODES {
        let url = format!("https://cardnyang.com/?mode={}", mode);
        let price_type = if *mode == "buying" { "buy" } else { "sell" };

        let html = match client.get(&url).send().await {
            Ok(resp) => match resp.text().await {
                Ok(text) => text,
                Err(e) => {
                    tracing::warn!("cardnyang {} text error: {}", mode, e);
                    continue;
                }
            },
            Err(e) => {
                tracing::warn!("cardnyang {} fetch error: {}", mode, e);
                continue;
            }
        };

        // Parse rows from the HTML table. Each row typically has a card code + price.
        for line in html.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Look for lines that contain a card code pattern
            if let Some(code_match) = card_code_re.find(trimmed) {
                let found_id = code_match.as_str().to_string();

                // If filtering by card_id, skip non-matching codes
                if let Some(filter_id) = card_id {
                    if !found_id.eq_ignore_ascii_case(filter_id) {
                        continue;
                    }
                }

                // Extract the first price number from the line
                if let Some(price_match) = price_re.find(trimmed) {
                    let price_str = price_match.as_str().replace(',', "");
                    if let Ok(price) = price_str.parse::<i64>() {
                        if price > 0 {
                            results.push(PriceItem {
                                source: "cardnyang".to_string(),
                                source_name: "카드냥".to_string(),
                                card_id: Some(found_id.clone()),
                                card_name_raw: found_id,
                                price,
                                price_type: price_type.to_string(),
                                url: Some(url.clone()),
                                fetched_at: fetched_at.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(results)
}
