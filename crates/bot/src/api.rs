use go_news_shared::{AddSubscriptionRequest, MarkPostedRequest, RssItem};
use log::{info, warn};
use reqwest::Client as HttpClient;
use std::error::Error;

#[derive(Clone)]
pub struct BackendApiClient {
    client: HttpClient,
    base_url: String,
}

impl BackendApiClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: HttpClient::new(),
            base_url,
        }
    }

    /// Fetches unposted items from the backend API.
    /// If `channel_id` is specified, returns items not yet delivered to that channel.
    pub async fn fetch_unposted_items(
        &self,
        channel_id: Option<&str>,
    ) -> Result<Vec<RssItem>, Box<dyn Error>> {
        let url = match channel_id {
            Some(cid) => format!("{}/items/unposted?channel_id={}", self.base_url, cid),
            None => format!("{}/items/unposted", self.base_url),
        };
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!("Backend returned error status: {}", response.status()).into());
        }

        let items: Vec<RssItem> = response.json().await?;
        Ok(items)
    }

    /// Sends a list of links to the backend to mark them as posted.
    /// If `channel_id` is specified, records delivery specifically for that channel.
    pub async fn mark_items_posted(
        &self,
        links: &[String],
        channel_id: Option<&str>,
    ) -> Result<(), Box<dyn Error>> {
        if links.is_empty() {
            return Ok(());
        }

        let url = format!("{}/items/mark-posted", self.base_url);
        let payload = MarkPostedRequest {
            links: links.to_vec(),
            channel_id: channel_id.map(str::to_string),
        };

        let response = self.client.post(&url).json(&payload).send().await?;

        if response.status().is_success() {
            info!(
                "Successfully marked {} item(s) as posted (channel: {:?}).",
                links.len(),
                channel_id
            );
        } else {
            warn!(
                "Failed to mark items as posted in backend. Status: {}",
                response.status()
            );
        }

        Ok(())
    }

    /// Registers a channel subscription with the backend.
    pub async fn register_subscription(
        &self,
        channel_id: &str,
        guild_id: Option<&str>,
    ) -> Result<(), Box<dyn Error>> {
        let url = format!("{}/subscriptions", self.base_url);
        let payload = AddSubscriptionRequest {
            channel_id: channel_id.to_string(),
            guild_id: guild_id.map(str::to_string),
        };

        let response = self.client.post(&url).json(&payload).send().await?;
        if !response.status().is_success() {
            warn!(
                "Failed to register channel subscription {} with backend. Status: {}",
                channel_id,
                response.status()
            );
        }
        Ok(())
    }
}
