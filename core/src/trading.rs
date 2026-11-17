//! Trading center: limited-item values from Rolimons (third party), clearly attributed.
//! Novastrap never invents values. Trade history/offers need the Roblox session cookie,
//! which Novastrap does not use, so they are not available.
//! Source: https://www.rolimons.com/itemapi/itemdetails (public item API).

use crate::error::{AppError, AppResult};
use crate::web::Api;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

pub const SOURCE_NAME: &str = "Rolimons";
pub const SOURCE_URL: &str = "https://www.rolimons.com/itemapi/itemdetails";
/// Rolimons asks API users to keep request rates low; values change slowly.
const CACHE: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ItemValue {
    pub id: u64,
    pub name: String,
    pub acronym: Option<String>,
    /// Recent average price (Robux).
    pub rap: Option<u64>,
    /// Rolimons' assigned value; None when they haven't valued it.
    pub value: Option<u64>,
    pub demand: Option<Level>,
    pub trend: Option<Trend>,
    pub projected: bool,
    pub hyped: bool,
    pub rare: bool,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    Terrible,
    Low,
    Normal,
    High,
    Amazing,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Trend {
    Lowering,
    Unstable,
    Stable,
    Raising,
    Fluctuating,
}

fn num(v: &Value) -> Option<i64> {
    v.as_i64()
}

fn opt_pos(v: &Value) -> Option<u64> {
    num(v).filter(|n| *n >= 0).map(|n| n as u64)
}

/// Rolimons row: [name, acronym, rap, value, default_value, demand, trend, projected, hyped, rare]
/// where -1 means "not set".
pub fn parse_row(id: u64, row: &[Value]) -> Option<ItemValue> {
    if row.len() < 10 {
        return None;
    }
    let level = |v: &Value| match num(v)? {
        0 => Some(Level::Terrible),
        1 => Some(Level::Low),
        2 => Some(Level::Normal),
        3 => Some(Level::High),
        4 => Some(Level::Amazing),
        _ => None,
    };
    let trend = |v: &Value| match num(v)? {
        0 => Some(Trend::Lowering),
        1 => Some(Trend::Unstable),
        2 => Some(Trend::Stable),
        3 => Some(Trend::Raising),
        4 => Some(Trend::Fluctuating),
        _ => None,
    };
    Some(ItemValue {
        id,
        name: row[0].as_str()?.to_owned(),
        acronym: row[1].as_str().filter(|s| !s.is_empty()).map(str::to_owned),
        rap: opt_pos(&row[2]),
        value: opt_pos(&row[3]),
        demand: level(&row[5]),
        trend: trend(&row[6]),
        projected: num(&row[7]) == Some(1),
        hyped: num(&row[8]) == Some(1),
        rare: num(&row[9]) == Some(1),
    })
}

#[derive(Deserialize)]
struct RawItems {
    success: bool,
    items: HashMap<String, Vec<Value>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueTable {
    pub source_name: &'static str,
    pub source_url: &'static str,
    /// Unix seconds when fetched.
    pub fetched_at: u64,
    pub items: Vec<ItemValue>,
}

pub async fn values(api: &Api) -> AppResult<ValueTable> {
    let raw: RawItems = api.get_json(SOURCE_URL, CACHE).await?;
    if !raw.success {
        return Err(AppError::UpstreamChanged("Rolimons reported failure".into()));
    }
    let mut items: Vec<ItemValue> = raw
        .items
        .iter()
        .filter_map(|(k, row)| parse_row(k.parse().ok()?, row))
        .collect();
    items.sort_by(|a, b| b.value.or(b.rap).cmp(&a.value.or(a.rap)));
    Ok(ValueTable { source_name: SOURCE_NAME, source_url: SOURCE_URL, fetched_at: crate::accounts::oauth::now_secs(), items })
}

/// Sum for a comparison side: uses value when Rolimons set one, otherwise RAP.
pub fn side_total(items: &[ItemValue]) -> u64 {
    items.iter().map(|i| i.value.or(i.rap).unwrap_or(0)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_rows() {
        let row: Vec<Value> = serde_json::from_value(json!(["Domino Crown", "DC", 5726071, 24000000, 24000000, 0, 2, -1, -1, 1])).unwrap();
        let v = parse_row(1031429, &row).unwrap();
        assert_eq!(v.acronym.as_deref(), Some("DC"));
        assert_eq!(v.value, Some(24_000_000));
        assert_eq!(v.demand, Some(Level::Terrible));
        assert_eq!(v.trend, Some(Trend::Stable));
        assert!(v.rare && !v.projected);

        let unvalued: Vec<Value> = serde_json::from_value(json!(["Red Baseball Cap", "", 1377, -1, 1377, -1, -1, -1, -1, -1])).unwrap();
        let u = parse_row(1, &unvalued).unwrap();
        assert_eq!(u.value, None, "-1 means not valued, never shown as a number");
        assert_eq!(u.demand, None);
        assert!(parse_row(1, &[json!("short")]).is_none());
    }

    #[test]
    fn totals_prefer_value() {
        let mk = |value, rap| ItemValue { id: 1, name: "x".into(), acronym: None, rap, value, demand: None, trend: None, projected: false, hyped: false, rare: false };
        assert_eq!(side_total(&[mk(Some(100), Some(50)), mk(None, Some(30)), mk(None, None)]), 130);
    }

    #[tokio::test]
    #[ignore]
    async fn live_values() {
        let api = Api::new(crate::roblox::deployment::http_client().unwrap());
        let t = values(&api).await.unwrap();
        println!("{} items from {}; top: {:?}", t.items.len(), t.source_name, t.items.first().map(|i| (&i.name, i.value)));
        assert!(t.items.len() > 1000);
    }
}
