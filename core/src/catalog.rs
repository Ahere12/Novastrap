//! Catalog: public marketplace search, item details, thumbnails and avatar previews.
//! Purchasing / wearing items needs the session cookie, so those link out to roblox.com.

use crate::accounts::ThumbList;
use crate::discover::encode_query;
use crate::error::{AppError, AppResult};
use crate::web::{Api, ttl};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItem {
    pub id: u64,
    pub item_type: String,
    #[serde(default)]
    pub asset_type: Option<u32>,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub creator_name: String,
    #[serde(default)]
    pub creator_has_verified_badge: bool,
    /// Robux price; None when not for sale.
    #[serde(default)]
    pub price: Option<u64>,
    #[serde(default)]
    pub lowest_price: Option<u64>,
    #[serde(default)]
    pub lowest_resale_price: Option<u64>,
    #[serde(default)]
    pub favorite_count: u64,
    #[serde(default)]
    pub item_restrictions: Vec<String>,
    #[serde(default)]
    pub total_quantity: Option<u64>,
    #[serde(default)]
    pub units_available_for_consumption: Option<u64>,
    #[serde(default)]
    pub is_off_sale: bool,
    #[serde(default)]
    pub item_created_utc: Option<String>,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPage {
    #[serde(default)]
    pub data: Vec<CatalogItem>,
    #[serde(default)]
    pub next_page_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    All,
    Accessories,
    Clothing,
    BodyParts,
    Animations,
    Collectibles,
}

impl Category {
    fn code(self) -> u32 {
        match self {
            Category::All => 1,
            Category::Collectibles => 2,
            Category::Clothing => 3,
            Category::BodyParts => 4,
            Category::Accessories => 11,
            Category::Animations => 12,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SortBy {
    Relevance,
    MostFavorited,
    Bestselling,
    RecentlyUpdated,
    PriceLowToHigh,
    PriceHighToLow,
}

impl SortBy {
    fn code(self) -> u32 {
        match self {
            SortBy::Relevance => 0,
            SortBy::MostFavorited => 1,
            SortBy::Bestselling => 2,
            SortBy::RecentlyUpdated => 3,
            SortBy::PriceLowToHigh => 4,
            SortBy::PriceHighToLow => 5,
        }
    }
}

pub fn search_url(keyword: &str, category: Category, sort: SortBy, cursor: Option<&str>) -> AppResult<String> {
    let k = keyword.trim();
    if k.len() > 60 {
        return Err(AppError::InvalidInput("Search for up to 60 characters".into()));
    }
    let mut url = format!(
        "https://catalog.roblox.com/v2/search/items/details?Keyword={}&Category={}&SortType={}&Limit=30",
        encode_query(k),
        category.code(),
        sort.code()
    );
    if let Some(c) = cursor {
        if !crate::servers::valid_cursor(c) {
            return Err(AppError::InvalidInput("invalid page cursor".into()));
        }
        url.push_str("&Cursor=");
        url.push_str(&encode_query(c));
    }
    Ok(url)
}

pub async fn attach_thumbnails(api: &Api, items: &mut [CatalogItem]) {
    let mut found: HashMap<(String, u64), String> = HashMap::new();
    for (kind, endpoint) in [("Asset", "assets?assetIds"), ("Bundle", "bundles/thumbnails?bundleIds")] {
        let ids: Vec<String> = items.iter().filter(|i| i.item_type == kind).map(|i| i.id.to_string()).collect();
        for chunk in ids.chunks(100) {
            let url = format!("https://thumbnails.roblox.com/v1/{endpoint}={}&size=150x150&format=Png", chunk.join(","));
            if let Ok(t) = api.get_json::<ThumbList>(&url, ttl::LONG).await {
                found.extend(t.data.into_iter().filter_map(|t| Some(((kind.to_string(), t.target_id), t.image_url?))));
            }
        }
    }
    for i in items {
        i.thumbnail_url = found.get(&(i.item_type.clone(), i.id)).cloned();
    }
}

pub async fn search(api: &Api, keyword: &str, category: Category, sort: SortBy, cursor: Option<&str>) -> AppResult<CatalogPage> {
    let mut page: CatalogPage = api.get_json(&search_url(keyword, category, sort, cursor)?, ttl::MEDIUM).await?;
    attach_thumbnails(api, &mut page.data).await;
    Ok(page)
}

/// Full-body avatar render for a user (public).
pub async fn avatar(api: &Api, user_id: u64) -> AppResult<Option<String>> {
    let url = format!("https://thumbnails.roblox.com/v1/users/avatar?userIds={user_id}&size=420x420&format=Png&isCircular=false");
    let t: ThumbList = api.get_json(&url, ttl::MEDIUM).await?;
    Ok(t.data.into_iter().find(|t| t.state == "Completed").and_then(|t| t.image_url))
}

pub fn item_url(item_type: &str, id: u64) -> String {
    match item_type {
        "Bundle" => format!("https://www.roblox.com/bundles/{id}"),
        _ => format!("https://www.roblox.com/catalog/{id}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_search_url() {
        let u = search_url("red hat", Category::Accessories, SortBy::Bestselling, None).unwrap();
        assert!(u.contains("Keyword=red%20hat&Category=11&SortType=2&Limit=30"));
        assert!(search_url("x", Category::All, SortBy::Relevance, Some("a&b")).is_err());
    }

    #[test]
    fn parses_item() {
        let i: CatalogItem = serde_json::from_str(r#"{"id":1365767,"itemType":"Asset","assetType":8,"name":"Valkyrie Helm","price":0,"lowestPrice":319795,"favoriteCount":667983,"itemRestrictions":["Limited"],"isOffSale":true,"creatorName":"Roblox"}"#).unwrap();
        assert_eq!(i.lowest_price, Some(319795));
        assert_eq!(item_url("Bundle", 5), "https://www.roblox.com/bundles/5");
    }

    #[tokio::test]
    #[ignore]
    async fn live_catalog() {
        let api = Api::new(crate::roblox::deployment::http_client().unwrap());
        let p = search(&api, "valkyrie", Category::Accessories, SortBy::Relevance, None).await.unwrap();
        println!("{} items; first {:?} thumb={:?}", p.data.len(), p.data[0].name, p.data[0].thumbnail_url.is_some());
        assert!(avatar(&api, 1).await.unwrap().is_some());
    }
}
