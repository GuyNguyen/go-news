use go_news_shared::{MarkPostedRequest, RssItem};
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

    /// Fetches all unposted items from the backend API.
    pub async fn fetch_unposted_items(&self) -> Result<Vec<RssItem>, Box<dyn Error>> {
        let url = format!("{}/items/unposted", self.base_url);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!("Backend returned error status: {}", response.status()).into());
        }

        let items: Vec<RssItem> = response.json().await?;
        Ok(items)
    }

    /// Sends a list of links to the backend to mark them as posted.
    pub async fn mark_items_posted(&self, links: &[String]) -> Result<(), Box<dyn Error>> {
        if links.is_empty() {
            return Ok(());
        }

        let url = format!("{}/items/mark-posted", self.base_url);
        let payload = MarkPostedRequest {
            links: links.to_vec(),
        };

        let response = self.client.post(&url).json(&payload).send().await?;

        if response.status().is_success() {
            info!("Successfully marked {} item(s) as posted.", links.len());
        } else {
            warn!(
                "Failed to mark items as posted in backend. Status: {}",
                response.status()
            );
        }

        Ok(())
    }
}
