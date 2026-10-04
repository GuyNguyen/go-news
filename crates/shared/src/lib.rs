use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct RssItem {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub link: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub pub_date: String,
    #[serde(default)]
    pub posted: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct MarkPostedRequest {
    pub links: Vec<String>,
    #[serde(default)]
    pub channel_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct SubscriptionItem {
    pub channel_id: String,
    pub guild_id: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub delivered_count: i64,
    #[serde(default)]
    pub last_delivered_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct AddSubscriptionRequest {
    pub channel_id: String,
    #[serde(default)]
    pub guild_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct ItemDeliveryStatus {
    pub title: String,
    pub link: String,
    pub pub_date: String,
    pub delivered_channels: Vec<String>,
    pub pending_channels: Vec<String>,
}

/// Parses an RSS date string into a normalized UTC DateTime.
/// Supports RFC 2822, RFC 3339 / ISO 8601, and standard web feeds.
pub fn parse_rss_date(date_str: &str) -> Option<DateTime<Utc>> {
    let trimmed = date_str.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Try RFC 2822 (standard RSS pubDate)
    if let Ok(dt) = DateTime::parse_from_rfc2822(trimmed) {
        return Some(dt.with_timezone(&Utc));
    }

    // Try RFC 3339 / ISO 8601 (common in Atom & Dublin Core)
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Some(dt.with_timezone(&Utc));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rss_item_serde() {
        let json_str = r#"{"title":"Go Tournament","link":"https://example.com/go","description":"News","pub_date":"2026-10-02"}"#;
        let item: RssItem = serde_json::from_str(json_str).expect("Failed to deserialize");
        assert_eq!(item.title, "Go Tournament");
        assert_eq!(item.link, "https://example.com/go");
        assert!(!item.posted);
    }

    #[test]
    fn test_parse_rss_date_formats() {
        // RFC 2822
        let dt1 = parse_rss_date("Sat, 05 Sep 2026 20:23:00 +0000");
        assert!(dt1.is_some());
        assert_eq!(dt1.unwrap().to_rfc3339(), "2026-09-05T20:23:00+00:00");

        // RFC 3339 / ISO 8601
        let dt2 = parse_rss_date("2026-09-05T20:23:00Z");
        assert!(dt2.is_some());
        assert_eq!(dt2.unwrap().to_rfc3339(), "2026-09-05T20:23:00+00:00");

        // Empty string
        assert!(parse_rss_date("").is_none());
        assert!(parse_rss_date("invalid date").is_none());
    }

    #[test]
    fn test_mark_posted_request_serde() {
        // Without channel_id (backwards compatibility)
        let json_legacy = r#"{"links":["https://example.com/1"]}"#;
        let req1: MarkPostedRequest = serde_json::from_str(json_legacy).unwrap();
        assert_eq!(req1.links, vec!["https://example.com/1"]);
        assert_eq!(req1.channel_id, None);

        // With channel_id
        let json_multi = r#"{"links":["https://example.com/1"],"channel_id":"123456789"}"#;
        let req2: MarkPostedRequest = serde_json::from_str(json_multi).unwrap();
        assert_eq!(req2.links, vec!["https://example.com/1"]);
        assert_eq!(req2.channel_id, Some("123456789".to_string()));
    }
}
