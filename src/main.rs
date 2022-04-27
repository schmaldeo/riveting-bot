#![feature(let_else)]
#![feature(decl_macro)]
// #![feature(type_alias_impl_trait)]

use std::sync::{Arc, Mutex};
use std::{env, fs};

use futures::stream::StreamExt;
use tracing_subscriber::EnvFilter;
use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::cluster::ShardScheme;
use twilight_gateway::{Cluster, Event};
use twilight_http::Client;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::Message;
use twilight_model::datetime::Timestamp;
use twilight_model::gateway::event::shard::Connected;
use twilight_model::gateway::payload::incoming::Ready;
use twilight_model::gateway::Intents;
use twilight_model::guild::Guild;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::Id;
use twilight_model::oauth::Application;
use twilight_model::user::CurrentUser;
use twilight_model::voice::VoiceState;
use utils::*;

use crate::commands::ChatCommands;
use crate::config::{BotConfig, Config};

mod commands;
mod config;
mod utils;

#[derive(Debug, Clone)]
pub struct Context {
    config: BotConfig,
    http: Arc<Client>,
    cluster: Arc<Cluster>,
    application: Arc<Application>,
    user: Arc<CurrentUser>,
    cache: Arc<InMemoryCache>,
    chat_commands: Arc<ChatCommands>,
    shard: Option<u64>,
}

impl Context {
    fn with_shard(mut self, id: u64) -> Self {
        self.shard = Some(id);
        self
    }
}

#[tokio::main]
async fn main() -> AnyResult<()> {
    // Load environment variables from `./.env` file, if any exists.
    dotenv::dotenv().ok();

    // Create a log file.
    let logfile = fs::File::create("./data/log.log").unwrap();

    // Initialize the logger to use `RUST_LOG` environment variable.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(false)
        .with_writer(Mutex::new(logfile))
        .compact()
        .init();

    // Load bot configuration file.
    let config = BotConfig::new(Config::load()?);

    // Get discord bot token from environment variable.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // Create an http client.
    let http = Arc::new(Client::new(token.to_owned()));

    // Start a gateway connection, technically even a single shard would be enough for this, but hey.
    let (cluster, mut events) = Cluster::builder(token, intents())
        .shard_scheme(ShardScheme::Auto)
        .http_client(Arc::clone(&http))
        .build()
        .await?;
    let cluster = Arc::new(cluster);

    // Start up the cluster.
    {
        let cluster = Arc::clone(&cluster);
        tokio::spawn(async move {
            cluster.up().await;
        });
    }

    // Spawn ctrl-c shutdown task.
    {
        let cluster = Arc::clone(&cluster);
        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Could not register ctrl+c handler");

            info!("Shutting down by ctrl-c");

            // cluster.down_resumable();
            cluster.down();

            println!("Ctrl-C");
        });
    }

    // Get the application info, such as its id and owner.
    let application = Arc::new(http.current_user_application().send().await?);

    // Get the bot user info.
    let user = Arc::new(http.current_user().send().await?);

    // Create a cache.
    let cache = Arc::new(InMemoryCache::new());

    // Initialize chat commands.
    let chat_commands = Arc::new(ChatCommands::new(&config.lock().unwrap().global.prefix));

    let ctx = Context {
        config,
        http,
        cluster,
        application,
        user,
        cache,
        chat_commands,
        shard: None,
    };

    // Process each event as they come in.
    while let Some((id, event)) = events.next().await {
        // Update the cache with the event.
        ctx.cache.update(&event);

        tokio::spawn(handle_event(ctx.clone().with_shard(id), event));
    }

    Ok(())
}

/// Main events handler.
async fn handle_event(ctx: Context, event: Event) -> AnyResult<()> {
    match event {
        Event::ShardConnected(c) => handle_shard_connected(&ctx, c).await,
        Event::Ready(r) => handle_ready(&ctx, *r).await,
        Event::GuildCreate(g) => handle_guild_create(&ctx, g.0).await,
        Event::InteractionCreate(i) => handle_interaction_create(&ctx, i.0).await,
        Event::MessageCreate(msg) => handle_message_create(&ctx, &msg.0).await,
        Event::VoiceStateUpdate(v) => handle_voice_state(&ctx, v.0).await,

        // Other events here...
        event => {
            println!("Event: {:?}", event.kind());
            debug!("Event: {:?}", event.kind());

            Ok(())
        },
    }
}

async fn handle_shard_connected(_ctx: &Context, connected: Connected) -> AnyResult<()> {
    info!(
        "Connected on shard {} with a heartbeat of {}",
        connected.shard_id, connected.heartbeat_interval
    );

    Ok(())
}

async fn handle_ready(_ctx: &Context, ready: Ready) -> AnyResult<()> {
    info!("Ready: '{}'", ready.user.name);

    Ok(())
}

async fn handle_guild_create(_ctx: &Context, guild: Guild) -> AnyResult<()> {
    println!("Guild: {:#?}", guild);
    info!("Guild: '{}'", guild.name);

    Ok(())
}

async fn handle_interaction_create(ctx: &Context, inter: Interaction) -> AnyResult<()> {
    match inter {
        Interaction::Ping(p) => {
            println!("{:#?}", p);
        },
        Interaction::ApplicationCommand(c) => {
            println!("{:#?}", c);
        },
        Interaction::ApplicationCommandAutocomplete(a) => {
            println!("{:#?}", a);
        },
        Interaction::MessageComponent(m) => {
            println!("{:#?}", m);
            let inter = ctx.http.interaction(ctx.application.id);
            let resp = InteractionResponse {
                kind: InteractionResponseType::UpdateMessage,
                data: Some(
                    twilight_util::builder::InteractionResponseDataBuilder::new()
                        .content("newcontent".to_string())
                        .build(),
                ),
            };
            inter.create_response(m.id, &m.token, &resp).exec().await?;

            println!("{:#?}", "DONE");
        },
        Interaction::ModalSubmit(s) => {
            println!("{:#?}", s);
        },
        i => todo!("not yet implemented: {:?}", i),
    };

    Ok(())
}

async fn handle_message_create(ctx: &Context, msg: &Message) -> AnyResult<()> {
    // Ignore bot users.
    anyhow::ensure!(!msg.author.bot, "Message sender is a bot");

    // Handle chat commands.
    if let Err(e) = ctx.chat_commands.process(ctx, msg).await {
        eprintln!("Error processing command: {}", e);
        error!("Error processing command: {}", e);
    }

    Ok(())
}

async fn handle_voice_state(ctx: &Context, voice: VoiceState) -> AnyResult<()> {
    println!("Voice: {:?}", voice);

    if voice.user_id == ctx.application.owner.as_ref().unwrap().id && voice.channel_id.is_some() {
        if let Some(guild_id) = voice.guild_id {
            // TODO Testing some stuff.
            if guild_id != Id::new(env::var("TEST_GUILD").unwrap().parse().unwrap()) {
                return Ok(());
            }

            ctx.http
                .guild_member(guild_id, voice.user_id)
                .send()
                .await?;

            let now = chrono::Utc::now();
            let until = now.timestamp() + 60;

            ctx.http
                .update_guild_member(guild_id, voice.user_id)
                .communication_disabled_until(Some(Timestamp::from_secs(until).unwrap()))
                .unwrap()
                .exec()
                .await?;

            // let muted = ctx
            //     .cache
            //     .member(guild_id, voice.user_id)
            //     .unwrap()
            //     .mute()
            //     .unwrap();
            // println!("1 {:?}", muted);
            // let m = ctx
            //     .http
            //     .guild_member(guild_id, voice.user_id)
            //     .exec()
            //     .await
            //     .unwrap()
            //     .model()
            //     .await?;
            // println!("2 {:#?}", m);
            // if !muted {
            //     let a = ctx
            //         .http
            //         .update_guild_member(guild_id, voice.user_id)
            //         .mute(true)
            //         .exec()
            //         .await?
            //         .model()
            //         .await?;
            //     println!("3 {:#?}", a);
            // }
        }
    }

    Ok(())
}

fn intents() -> Intents {
    #[cfg(feature = "all-intents")]
    return Intents::all();
    Intents::MESSAGE_CONTENT
        | Intents::GUILD_MESSAGES
        | Intents::GUILD_MESSAGE_REACTIONS
        | Intents::GUILD_MEMBERS
        | Intents::GUILD_VOICE_STATES
        | Intents::DIRECT_MESSAGES
        | Intents::DIRECT_MESSAGE_REACTIONS
}
