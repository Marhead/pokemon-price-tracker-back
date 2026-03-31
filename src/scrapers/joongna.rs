use crate::models::price::PriceItem;
use anyhow::Result;

pub async fn fetch_listings(card_name: &str) -> Result<Vec<PriceItem>> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; PokemonPriceTracker/1.0)")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let encoded_name = urlencoding::encode(card_name).into_owned();
    let url = format!("https://web.joongna.com/search/{}", encoded_name);
    let fetched_at = chrono::Utc::now().to_rfc3339();

    let html = client
        .get(&url)
        .send()
        .await?
        .text()
        .await?;

    let mut results: Vec<PriceItem> = Vec::new();

    // Find __NEXT_DATA__ script tag
    let marker = r#"<script id="__NEXT_DATA__" type="application/json">"#;
    if let Some(start) = html.find(marker) {
        let content_start = start + marker.len();
        if let Some(end) = html[content_start..].find("</script>") {
            let json_str = &html[content_start..content_start + end];
            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(json_str) {
                extract_joongna_items(&json_val, card_name, &url, &fetched_at, &mut results);
            }
        }
    }

    Ok(results)
}

fn extract_joongna_items(
    json_val: &serde_json::Value,
    card_name: &str,
    source_url: &str,
    fetched_at: &str,
    results: &mut Vec<PriceItem>,
) {
    // Navigate through various possible paths in Next.js page data
    let possible_paths: Vec<Vec<&str>> = vec![
        vec!["props", "pageProps", "data", "list"],
        vec!["props", "pageProps", "initialData", "list"],
        vec!["props", "pageProps", "searchResult", "data"],
        vec!["props", "pageProps", "products"],
    ];

    for path in &possible_paths {
        let mut current = json_val;
        for key in path {
            if let Some(next) = current.get(key) {
                current = next;
            } else {
                current = json_val; // reset
                break;
            }
        }

        if let Some(items) = current.as_array() {
            for item in items {
                try_extract_joongna_item(item, card_name, source_url, fetched_at, results);
            }
            if !results.is_empty() {
                return;
            }
        }
    }

    // Fallback: traverse entire JSON looking for arrays of product-like objects
    find_items_recursive(json_val, card_name, source_url, fetched_at, results, 0);
}

fn try_extract_joongna_item(
    item: &serde_json::Value,
    card_name: &str,
    source_url: &str,
    fetched_at: &str,
    results: &mut Vec<PriceItem>,
) {
    let title = item
        .get("title")
        .or_else(|| item.get("name"))
        .or_else(|| item.get("productName"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if title.is_empty()
        || !title
            .to_lowercase()
            .contains(&card_name.to_lowercase())
    {
        return;
    }

    let price = item
        .get("price")
        .or_else(|| item.get("salePrice"))
        .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| {
            s.replace([',', '원'], "").trim().parse::<i64>().ok()
        })))
        .unwrap_or(0);

    if price <= 0 {
        return;
    }

    let item_url = item
        .get("url")
        .or_else(|| item.get("productUrl"))
        .and_then(|v| v.as_str())
        .map(|s| {
            if s.starts_with("http") {
                s.to_string()
            } else {
                format!("https://web.joongna.com{}", s)
            }
        })
        .or_else(|| {
            item.get("seq")
                .or_else(|| item.get("id"))
                .and_then(|v| v.as_i64())
                .map(|id| format!("https://web.joongna.com/product/{}", id))
        })
        .or_else(|| Some(source_url.to_string()));

    results.push(PriceItem {
        source: "joongna".to_string(),
        source_name: "중고나라".to_string(),
        card_id: None,
        card_name_raw: title.to_string(),
        price,
        price_type: "used".to_string(),
        url: item_url,
        fetched_at: fetched_at.to_string(),
    });
}

fn find_items_recursive(
    val: &serde_json::Value,
    card_name: &str,
    source_url: &str,
    fetched_at: &str,
    results: &mut Vec<PriceItem>,
    depth: usize,
) {
    if depth > 8 {
        return;
    }
    match val {
        serde_json::Value::Array(arr) => {
            // If first element looks like a product, try extracting
            if let Some(first) = arr.first() {
                if first.get("title").is_some() || first.get("price").is_some() {
                    for item in arr {
                        try_extract_joongna_item(item, card_name, source_url, fetched_at, results);
                    }
                    return;
                }
            }
            for item in arr {
                find_items_recursive(item, card_name, source_url, fetched_at, results, depth + 1);
            }
        }
        serde_json::Value::Object(map) => {
            for (_, v) in map {
                find_items_recursive(v, card_name, source_url, fetched_at, results, depth + 1);
            }
        }
        _ => {}
    }
}
