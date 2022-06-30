#![feature(let_else)]
#![feature(decl_macro)]
#![feature(pattern)]
#![feature(associated_type_bounds)]
#![feature(option_get_or_insert_default)]
#![allow(dead_code)]
#![allow(clippy::significant_drop_in_scrutinee)]

use std::sync::{Arc, Mutex};
use std::{env, fs};

use futures::stream::StreamExt;
use tracing::Level;
use tracing_subscriber::EnvFilter;
use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::{Cluster, Event};
use twilight_http::Client;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::{Message, Reaction};
use twilight_model::gateway::event::shard::Connected;
use twilight_model::gateway::payload::incoming::{
    MessageDelete, MessageDeleteBulk, MessageUpdate, Ready,
};
use twilight_model::gateway::Intents;
use twilight_model::guild::Guild;
use twilight_model::id::Id;
use twilight_model::oauth::Application;
use twilight_model::user::CurrentUser;
use twilight_model::voice::VoiceState;
use twilight_standby::Standby;

use crate::commands::admin::scheduler::handle_timer;
use crate::commands::{ChatCommands, CommandCall, CommandError};
use crate::config::Config;
use crate::utils::prelude::*;
use crate::utils::Shenanigans;

mod commands;
mod config;
mod parser;
mod utils;

#[derive(Debug, Clone)]
pub struct Context {
    config: Arc<Mutex<Config>>,
    http: Arc<Client>,
    cluster: Arc<Cluster>,
    application: Arc<Application>,
    user: Arc<CurrentUser>,
    cache: Arc<InMemoryCache>,
    standby: Arc<Standby>,
    chat_commands: Arc<ChatCommands>,
    shard: Option<u64>,
}

impl Context {
    const fn with_shard(mut self, id: u64) -> Self {
        self.shard = Some(id);
        self
    }
}

#[tokio::main]
async fn main() -> AnyResult<()> {
    // Load environment variables from `./.env` file, if any exists.
    dotenv::dotenv().ok();

    // Create data folder if it doesn't exist yet.
    std::fs::create_dir_all("./data/")
        .map_err(|e| anyhow::anyhow!("Failed to create data folder: {}", e))?;

    // Create a log file or truncate an existing one.
    let logfile = fs::File::create("./data/log.log")
        .map_err(|e| anyhow::anyhow!("Failed to create log file: {}", e))?;

    // Initialize the logger to use `RUST_LOG` environment variable.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(Level::DEBUG.into())
                .from_env()?,
        )
        .with_ansi(false)
        .with_writer(Mutex::new(logfile))
        .compact()
        .init();

    // Load bot configuration file.
    let config = Arc::new(Mutex::new(Config::load()?));

    // Get discord bot token from environment variable.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // Create an http client.
    let http = Arc::new(Client::new(token.to_owned()));

    // Start a gateway connection, technically even a single shard would be enough for this, but hey.
    let (cluster, mut events) = Cluster::builder(token, intents())
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

    // Create a standby instance.
    let standby = Arc::new(Standby::new());

    // Initialize chat commands.
    let chat_commands = Arc::new(ChatCommands::new());

    let ctx = Context {
        config,
        http,
        cluster,
        application,
        user,
        cache,
        standby,
        chat_commands,
        shard: None,
    };

    // Spawn community event handler.
    tokio::spawn(handle_timer(ctx.clone(), 60 * 5));

    // Process each event as they come in.
    while let Some((id, event)) = events.next().await {
        // Update the cache with the event.
        ctx.cache.update(&event);

        // Update standby events.
        let processed = ctx.standby.process(&event);
        log_processed(processed);

        tokio::spawn(handle_event(ctx.clone().with_shard(id), event));
    }

    Ok(())
}

/// Main events handler.
async fn handle_event(ctx: Context, event: Event) -> AnyResult<()> {
    let result = match event {
        Event::ShardConnected(c) => handle_shard_connected(&ctx, c).await,
        Event::Ready(r) => handle_ready(&ctx, *r).await,
        Event::GuildCreate(g) => handle_guild_create(&ctx, g.0).await,
        Event::InteractionCreate(i) => handle_interaction_create(&ctx, i.0).await,
        Event::MessageCreate(mc) => handle_message_create(&ctx, mc.0).await,
        Event::MessageUpdate(mu) => handle_message_update(&ctx, *mu).await,
        Event::MessageDelete(md) => handle_message_delete(&ctx, md).await,
        Event::MessageDeleteBulk(mdb) => handle_message_delete_bulk(&ctx, mdb).await,
        Event::ReactionAdd(r) => handle_reaction_add(&ctx, r.0).await,
        Event::ReactionRemove(r) => handle_reaction_remove(&ctx, r.0).await,
        Event::VoiceStateUpdate(v) => handle_voice_state(&ctx, v.0).await,

        // Other events here...
        event => {
            println!("Event: {:?}", event.kind());
            debug!("Event: {:?}", event.kind());

            Ok(())
        },
    };

    if let Err(e) = result {
        println!("Event error: {e}");
        error!("Event error: {e}");
    }

    Ok(())
}

async fn handle_shard_connected(_ctx: &Context, connected: Connected) -> AnyResult<()> {
    info!(
        "Connected on shard {} with a heartbeat of {}",
        connected.shard_id, connected.heartbeat_interval
    );

    Ok(())
}

async fn handle_ready(_ctx: &Context, ready: Ready) -> AnyResult<()> {
    println!("Ready: '{}'", ready.user.name);
    info!("Ready: '{}'", ready.user.name);

    Ok(())
}

async fn handle_guild_create(_ctx: &Context, guild: Guild) -> AnyResult<()> {
    println!("Guild: {}", guild.name);
    info!("Guild: '{}'", guild.name);

    Ok(())
}

async fn handle_interaction_create(_ctx: &Context, _inter: Interaction) -> AnyResult<()> {
    // use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
    // match inter {
    //     Interaction::Ping(p) => {
    //         println!("{:#?}", p);
    //     },
    //     Interaction::ApplicationCommand(c) => {
    //         println!("{:#?}", c);
    //     },
    //     Interaction::ApplicationCommandAutocomplete(a) => {
    //         println!("{:#?}", a);
    //     },
    //     Interaction::MessageComponent(m) => {
    //         println!("{:#?}", m);
    //         let inter = ctx.http.interaction(ctx.application.id);
    //         let resp = InteractionResponse {
    //             kind: InteractionResponseType::UpdateMessage,
    //             data: Some(
    //                 twilight_util::builder::InteractionResponseDataBuilder::new()
    //                     .content("newcontent".to_string())
    //                     .build(),
    //             ),
    //         };
    //         inter.create_response(m.id, &m.token, &resp).exec().await?;

    //         println!("{:#?}", "DONE");
    //     },
    //     Interaction::ModalSubmit(s) => {
    //         println!("{:#?}", s);
    //     },
    //     i => todo!("not yet implemented: {:?}", i),
    // };

    Ok(())
}

async fn handle_message_create(ctx: &Context, msg: Message) -> AnyResult<()> {
    // Ignore bot users.
    if msg.author.bot {
        trace!("Message sender is a bot '{}'", msg.author.name);
        return Ok(());
    }

    // Handle chat commands.
    if let Err(e) = ctx.chat_commands.process(ctx, &msg).await {
        match e {
            // Message was not a command. Check if it should update a bot message content.
            CommandError::NotPrefixed => {
                update_bot_content(ctx, msg).await?;
            },

            // Log processing errors.
            e => {
                eprintln!("Error processing command: {}", e);
                error!("Error processing command: {}", e);

                if let Ok(id) = env::var("DISCORD_BOTDEV_CHANNEL") {
                    // On a bot dev channel, reply with error message.
                    let bot_dev = Id::new(id.parse()?);

                    if msg.channel_id == bot_dev {
                        ctx.http
                            .create_message(bot_dev)
                            .reply(msg.id)
                            .content(&e.to_string())?
                            .send()
                            .await?;
                    }
                }
            },
        }
    }

    Ok(())
}

async fn handle_message_update(ctx: &Context, mu: MessageUpdate) -> AnyResult<()> {
    // TODO Check if updated message is something that should update content from the bot.

    let text = mu.content.unwrap_or_default();
    let ccall = CommandCall::parse_from(ctx, mu.guild_id, &text).unwrap();

    println!("{}", ccall);

    Ok(())
}

async fn handle_message_delete(ctx: &Context, md: MessageDelete) -> AnyResult<()> {
    let Some(guild_id) = md.guild_id else {
        return Ok(());
    };

    let mut lock = ctx.config.lock().unwrap();

    let Some(settings) = lock.guild_mut(guild_id) else {
        return Ok(());
    };

    let key = format!("{}.{}", md.channel_id, md.id);
    settings.reaction_roles.remove(&key);

    lock.write_guild(guild_id)?;

    Ok(())
}

async fn handle_message_delete_bulk(ctx: &Context, mdb: MessageDeleteBulk) -> AnyResult<()> {
    let message_delete_with = |id| MessageDelete {
        channel_id: mdb.channel_id,
        guild_id: mdb.guild_id,
        id,
    };

    for id in mdb.ids {
        handle_message_delete(ctx, message_delete_with(id)).await?; // Haxxx.
    }

    Ok(())
}

async fn handle_reaction_add(ctx: &Context, reaction: Reaction) -> AnyResult<()> {
    let Some(guild_id) = reaction.guild_id else {
        return Ok(());
    };

    let user = match reaction.member {
        Some(m) => m.user,
        None => match ctx.cache.user(reaction.user_id) {
            Some(m) => m.to_owned(),
            None => ctx.http.user(reaction.user_id).send().await?,
        },
    };

    // Ignore reactions from bots.
    if user.bot {
        return Ok(());
    }

    // Check if message is cached.
    if let Some(msg) = ctx.cache.message(reaction.message_id) {
        // Ignore if message is not from this bot.
        if msg.author() != ctx.user.id {
            return Ok(());
        }
    }

    let add_roles = {
        let lock = ctx.config.lock().unwrap();

        let Some(settings) = lock.guild(guild_id) else {
            return Ok(());
        };

        let key = format!("{}.{}", reaction.channel_id, reaction.message_id);

        let Some(map) = settings.reaction_roles.get(&key) else {
            return Ok(());
        };

        map.iter()
            .filter(|rr| utils::reaction_type_eq(&rr.emoji, &reaction.emoji))
            .map(|rr| rr.role)
            .collect::<Vec<_>>()
    };

    info!("Adding roles for '{}'", user.name);
    if add_roles.is_empty() {
        return Err(anyhow::anyhow!("List of roles should not be empty"));
    }

    for role_id in add_roles {
        ctx.http
            .add_guild_member_role(guild_id, reaction.user_id, role_id)
            .exec()
            .await?;
    }

    Ok(())
}

async fn handle_reaction_remove(ctx: &Context, reaction: Reaction) -> AnyResult<()> {
    let Some(guild_id) = reaction.guild_id else {
        return Ok(());
    };

    let user = match reaction.member {
        Some(m) => m.user,
        None => match ctx.cache.user(reaction.user_id) {
            Some(m) => m.to_owned(),
            None => ctx.http.user(reaction.user_id).send().await?,
        },
    };

    // Ignore reactions from bots.
    if user.bot {
        return Ok(());
    }

    // Check if message is cached.
    if let Some(msg) = ctx.cache.message(reaction.message_id) {
        // Ignore if message is not from this bot.
        if msg.author() != ctx.user.id {
            return Ok(());
        }
    }

    let remove_roles = {
        let lock = ctx.config.lock().unwrap();

        let Some(settings) = lock.guild(guild_id) else {
            return Ok(());
        };

        let key = format!("{}.{}", reaction.channel_id, reaction.message_id);

        let Some(map) = settings.reaction_roles.get(&key) else {
            trace!("No reaction-roles config for key '{key}'");
            return Ok(());
        };

        map.iter()
            .filter(|rr| utils::reaction_type_eq(&rr.emoji, &reaction.emoji))
            .map(|rr| rr.role)
            .collect::<Vec<_>>()
    };

    info!("Removing roles for '{}'", user.name);
    if remove_roles.is_empty() {
        return Err(anyhow::anyhow!("List of roles should not be empty"));
    }

    for role_id in remove_roles {
        ctx.http
            .remove_guild_member_role(guild_id, reaction.user_id, role_id)
            .exec()
            .await?;
    }

    Ok(())
}

async fn handle_voice_state(_ctx: &Context, voice: VoiceState) -> AnyResult<()> {
    println!("{voice:#?}",);

    Ok(())
}

/// Check if message event should update a bot message content.
async fn update_bot_content(ctx: &Context, msg: Message) -> AnyResult<()> {
    // Ignore if message is not a reply or it is not in a guild.
    let (Some(replied), Some(guild_id)) = (msg.referenced_message.as_ref(), msg.guild_id) else {
        return Ok(());
    };

    // Ignore if replied message is not from a bot.
    if !replied.author.bot {
        return Ok(());
    }

    // Ignore if member is not admin.
    if !commands::sender_has_admin(ctx, &msg, guild_id).await? {
        return Ok(());
    }

    // Update a reaction-roles message, if found.
    let update = {
        let lock = ctx.config.lock().unwrap();

        lock.guild(guild_id).map_or(false, |settings| {
            let key = format!("{}.{}", replied.channel_id, replied.id);
            settings.reaction_roles.contains_key(&key)
        })
    };

    if update {
        // Edit message content
        ctx.http
            .update_message(replied.channel_id, replied.id)
            .content(Some(&msg.content))?
            .send()
            .await?;

        ctx.http
            .delete_message(msg.channel_id, msg.id)
            .exec()
            .await?;
    }

    Ok(())
}

fn intents() -> Intents {
    #[cfg(feature = "all-intents")]
    return Intents::all();
    Intents::MESSAGE_CONTENT
        | Intents::GUILDS
        | Intents::GUILD_MESSAGES
        | Intents::GUILD_MESSAGE_REACTIONS
        | Intents::GUILD_MEMBERS
        | Intents::GUILD_PRESENCES
        | Intents::GUILD_VOICE_STATES
        | Intents::DIRECT_MESSAGES
        | Intents::DIRECT_MESSAGE_REACTIONS
}

fn log_processed(p: twilight_standby::ProcessResults) {
    if p.dropped() + p.fulfilled() + p.matched() + p.sent() > 0 {
        debug!(
            "Standby: {{ m: {}, d: {}, f: {}, s: {} }}",
            p.matched(),
            p.dropped(),
            p.fulfilled(),
            p.sent(),
        );
    }
}
