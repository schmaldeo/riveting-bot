use std::env;
use std::error::Error;
use std::sync::Arc;

use futures::stream::StreamExt;
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::cluster::{Cluster, ShardScheme};
use twilight_gateway::Event;
use twilight_http::Client as HttpClient;
use twilight_model::gateway::Intents;
use utils::*;

mod commands;
mod roles;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Create a log file.
    let logfile = std::fs::File::create("log.log").unwrap();

    // Initialize the logger to use `RUST_LOG` environment variable.
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(std::sync::Mutex::new(logfile))
        .compact()
        .init();
    // tracing_subscriber::fmt().compact().init();
    // tracing_log::LogTracer::init()?;

    // Load environment variables from `./.env` file, if any exists.
    dotenv::dotenv().ok();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // Use intents to only receive guild message events.
    let (cluster, mut events) = Cluster::builder(token.to_owned(), intents())
        .shard_scheme(ShardScheme::Auto)
        .build()
        .await?;
    let cluster = Arc::new(cluster);

    // Start all shards in the cluster in the background.
    {
        let cluster_spawn = Arc::clone(&cluster);
        tokio::spawn(async move {
            cluster_spawn.up().await;
        });
    }

    {
        let cluster_spawn = Arc::clone(&cluster);
        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Could not register ctrl+c handler");
            info!("Shutting down");
            cluster_spawn.down();
            println!("Ctrl-C");
        });
    }

    // HTTP is separate from the gateway, so create a new client.
    let http = Arc::new(HttpClient::new(token));

    // Since we only care about new messages, make the cache only cache new messages.
    let cache = InMemoryCache::builder()
        .resource_types(ResourceType::MESSAGE)
        .build();

    // Process each event as they come in.
    while let Some((shard_id, event)) = events.next().await {
        // Update the cache with the event.
        cache.update(&event);

        tokio::spawn(handle_event(shard_id, event, Arc::clone(&http)));
    }

    Ok(())
}

async fn handle_event(
    shard_id: u64,
    event: Event,
    http: Arc<HttpClient>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match event {
        Event::MessageCreate(msg) if msg.content == "!ping" => {
            http.create_message(msg.channel_id)
                .content("Pong!")?
                .exec()
                .await?;
        }
        Event::ShardConnected(_) => {
            println!("Connected on shard {}", shard_id);
        }
        // Other events here...
        _ => {}
    }

    Ok(())
}

fn intents() -> Intents {
    Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT
}
