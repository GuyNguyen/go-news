# go-news

An RSS news feed aggregator and Discord notification bot built with Rust, specifically designed for Go (Baduk / Weiqi) news (such as [igomely.com](https://www.igomely.com)).

The project is structured as a Cargo workspace with a decoupled architecture: an Actix-web backend handles RSS fetching, deduplication, and SQLite persistence, while a Serenity-based Discord bot polls the backend and posts formatted embeds to one or more Discord channels.

---

## Features

- **Automated RSS Ingestion**: Periodically polls RSS feeds and deduplicates articles.
- **Anti-Spam Startup**: Automatically syncs existing feed items on fresh initialization to prevent flooding Discord channels with old news.
- **Dynamic Multi-Server Subscriptions**: Supports self-serve Discord Slash Commands (`/subscribe`, `/unsubscribe`, `/news-status`) so server admins can manage their own channels.
- **Multi-Channel Delivery Tracking**: Tracks delivered articles per channel/server independently so channels don't miss articles or receive duplicates.
- **Auto-Pruning**: Automatically cleans up old articles based on a configurable retention period.
- **Rich Discord Embeds**: Formats news posts with title, summary, publication date, and links.
- **Single Central Configuration**: Configured via `config.toml` (or entirely dynamic via slash commands).
- **Docker Ready**: Includes `docker-compose.yml` and Dockerfiles for zero-hassle deployment.

---

## Architecture

The repository is organized into three Rust crates:

- **`crates/shared`**: Shared data models (`RssItem`, `SubscriptionItem`, `MarkPostedRequest`) and feed date parsing helpers.
- **`crates/backend`**: REST API built with [Actix-web](https://actix.rs/) and [SQLx](https://github.com/launchbadge/sqlx) (SQLite in WAL mode). Responsible for feed polling, database storage, delivery status, and pruning.
- **`crates/bot`**: Discord bot built with [Serenity](https://github.com/serenity-rs/serenity). Polls the backend API for unposted articles, publishes embeds to configured channels, and reports delivery status.

---

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/) (version 1.75+) **OR** [Docker](https://www.docker.com/) and [Docker Compose](https://docs.docker.com/compose/)
- A [Discord Bot Token](https://discord.com/developers/applications) with message sending permissions

### 1. Configuration

Copy the template configuration file:

```bash
cp config.example.toml config.toml
```

Edit `config.toml` to specify your Discord bot token and target channel ID(s):

```toml
[backend]
database_url = "sqlite://data/go_news.db?mode=rwc"
feed_url = "https://www.igomely.com/feed"
check_interval_seconds = 1800
retention_days = 60
host = "0.0.0.0"
port = 8080

[bot]
api_url = "http://backend:8080"
check_interval_seconds = 60
discord_token = "YOUR_DISCORD_BOT_TOKEN"
channel_ids = [123456789012345678]
```

> **Note:** For local development outside of Docker, set `api_url = "http://localhost:8080"`.

---

### 2. Running with Docker Compose (Recommended)

Start both the backend service and the Discord bot containerized:

```bash
docker compose up -d --build
```

View logs:

```bash
docker compose logs -f
```

---

### 3. Running Locally with Cargo

1. Start the backend:
   ```bash
   cargo run -p go-news-backend
   ```

2. In a separate terminal, start the bot:
   ```bash
   cargo run -p go-news-bot
   ```

---

## Running Tests

Run the test suite across all workspace crates:

```bash
cargo test
```

---

## Backend API Overview

The backend exposes the following REST endpoints on port `8080`:

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/health` | Health check probe |
| `GET` | `/items` | List all stored news items |
| `GET` | `/items/unposted` | Get unposted items (optional query `?channel_id=<id>`) |
| `POST` | `/items/mark-posted` | Mark items as posted for a channel |
| `POST` | `/items/init-posted` | Initialize existing items as posted to prevent historical spam |
| `GET` | `/items/delivery-status` | Overview of cross-channel article delivery |
| `POST` | `/force-check` | Manually trigger an immediate RSS feed poll |
| `GET` | `/subscriptions` | List registered channel subscriptions |
| `POST` | `/subscriptions` | Register a new channel subscription |
| `DELETE`| `/subscriptions/{channel_id}` | Remove a channel subscription |

---

## License

This project is licensed under the [MIT License](LICENSE).
