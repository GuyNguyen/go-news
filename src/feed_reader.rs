use std::env;
use reqwest::Client;
use feed_rs::parser;
use feed_rs::model::Feed;


pub async fn feed_reader() -> Result<Feed, Box<dyn std::error::Error>> {
    let link = env::var("LINK").expect("LINK environment variable not set");
    let client = Client::new();

    let response = client.get(&link).send().await?;
    let xml = response.text().await?;

    let feed = parser::parse(xml.as_bytes())?;

    // for entry in &feed.entries {
    //     if let Some(link) = entry.links.first() {
    //         println!("Link: {}", link.href);
    //     }

    // }

    Ok(feed)
}
