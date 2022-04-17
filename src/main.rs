#![feature(let_else)]
#![feature(decl_macro)]

use std::io::prelude::*;
use std::sync::{Arc, Mutex};
use std::{env, fs};

use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;
use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::cluster::ShardScheme;
use twilight_gateway::{Cluster, Event};
use twilight_http::Client;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::Message;
use twilight_model::gateway::event::shard::Connected;
use twilight_model::gateway::payload::incoming::Ready;
use twilight_model::gateway::Intents;
use twilight_model::guild::Guild;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::oauth::Application;
use twilight_model::user::CurrentUser;
use twilight_model::voice::VoiceState;
use utils::*;

use crate::commands::ChatCommands;

mod commands;
mod utils;

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
struct Config {
    prefix: char,
}

impl Default for Config {
    fn default() -> Self {
        Self { prefix: '!' }
    }
}

#[derive(Debug, Clone)]
pub struct Context {
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
    // tracing_subscriber::fmt().compact().init();
    // tracing_log::LogTracer::init()?;

    load_bot_config()?;

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // Create an http client.
    let http = Arc::new(Client::new(token.to_owned()));

    // Technically even a single shard would be enough, but hey.
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
    let application = {
        let response = http.current_user_application().exec().await?;
        Arc::new(response.model().await?)
    };

    // Get the bot user info.
    let user = {
        let response = http.current_user().exec().await?;
        Arc::new(response.model().await?)
    };

    // Create a cache.
    let cache = Arc::new(InMemoryCache::new());

    let ctx = Context {
        http,
        cluster,
        application,
        user,
        cache,
        chat_commands: Arc::new(ChatCommands::new("!").await),
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
        Event::MessageCreate(msg) => handle_message_create(&ctx, msg.0).await,
        Event::VoiceStateUpdate(v) => handle_voice_state(&ctx, v.0).await,

        // Other events here...
        event => {
            println!("Event: {:?}", event.kind());
            debug!("Event: {:?}", event.kind());

            Ok(())
        }
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
    // let commands = commands::foo();
    // let interaction = ctx.http.interaction(ctx.application.id);
    // interaction
    //     .set_global_commands(&commands)
    //     .exec()
    //     .await
    //     .unwrap();

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
        }
        Interaction::ApplicationCommand(c) => {
            println!("{:#?}", c);
        }
        Interaction::ApplicationCommandAutocomplete(a) => {
            println!("{:#?}", a);
        }
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
        }
        Interaction::ModalSubmit(s) => {
            println!("{:#?}", s);
        }
        i => todo!("not yet implemented: {:?}", i),
    };

    Ok(())
}

async fn handle_message_create(ctx: &Context, msg: Message) -> AnyResult<()> {
    // Ignore bot users.
    anyhow::ensure!(!msg.author.bot, "Message sender is a bot");

    // Handle chat commands.
    if let Err(e) = ctx.chat_commands.process(ctx, &msg).await {
        eprintln!("Error processing command: {}", e);
        error!("Error processing command: {}", e);
    }

    Ok(())
}

async fn handle_voice_state(_ctx: &Context, voice: VoiceState) -> AnyResult<()> {
    // if v.user_id == ctx.application.owner.id && v.channel_id.is_some() {
    //     if let Some(gid) = v.guild_id {
    //         let muted = ctx.cache.member(gid, v.user_id).unwrap().mute().unwrap();
    //         println!("1 {:?}", muted);
    //         let m = ctx
    //             .http
    //             .guild_member(gid, v.user_id)
    //             .exec()
    //             .await
    //             .unwrap()
    //             .model()
    //             .await?;
    //         println!("2 {:#?}", m);
    //         if !muted {
    //             let a = ctx
    //                 .http
    //                 .update_guild_member(gid, v.user_id)
    //                 .mute(true)
    //                 .exec()
    //                 .await?
    //                 .model()
    //                 .await?;
    //             println!("3 {:#?}", a);
    //         }
    //     }
    // }

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
    // | Intents::GUILD_WEBHOOKS
    // | Intents::GUILD_INTEGRATIONS
}

fn load_bot_config() -> AnyResult<Config> {
    let mut config = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("./data/bot.toml")?;

    let mut cfg = String::new();
    config.read_to_string(&mut cfg)?;

    match toml::from_str(&cfg) {
        Ok(c) => Ok(c),
        Err(e) => {
            debug!("Could not load config: {}", e);
            info!("Creating a default config");

            let def = Config::default();
            config.write_all(toml::to_vec(&def)?.as_slice())?;

            Ok(def)
        }
    }
}
