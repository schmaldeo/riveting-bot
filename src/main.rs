#![feature(associated_type_bounds)]
#![feature(associated_type_defaults)]
#![feature(decl_macro)]
#![feature(iter_intersperse)]
#![feature(iterator_try_collect)]
#![feature(iterator_try_reduce)]
#![feature(option_get_or_insert_default)]
#![feature(pattern)]
#![feature(trait_alias)]
#![feature(async_fn_in_trait)]
#![allow(dead_code)]
#![allow(clippy::significant_drop_in_scrutinee)]

use std::sync::{Arc, Mutex};
use std::{env, fs};

use futures::stream::StreamExt;
use tracing::Level;
use tracing_subscriber::EnvFilter;
use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::{Cluster, Event};
use twilight_http::client::InteractionClient;
use twilight_http::Client;
use twilight_model::application::interaction::{Interaction, InteractionData};
use twilight_model::channel::{Message, Reaction};
use twilight_model::gateway::event::shard::Connected;
use twilight_model::gateway::payload::incoming::{
    MessageDelete, MessageDeleteBulk, MessageUpdate, Ready,
};
use twilight_model::gateway::Intents;
use twilight_model::guild::Guild;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;
use twilight_model::oauth::Application;
use twilight_model::user::CurrentUser;
use twilight_model::voice::VoiceState;
use twilight_standby::Standby;

use crate::commands_v2::{CommandError, Commands};
use crate::config::Config;
use crate::utils::prelude::*;

mod commands_v2;

// mod commands;
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
    commands: Arc<Commands>,
    shard: Option<u64>,
}

impl Context {
    /// This context with the provided shard id.
    const fn with_shard(mut self, id: u64) -> Self {
        self.shard = Some(id);
        self
    }

    /// Shortcut for `self.http.interaction(self.application.id)`.
    pub fn interaction(&self) -> InteractionClient {
        self.http.interaction(self.application.id)
    }

    /// Return currently active classic command prefix, either global prefix or a guild specific one.
    pub fn classic_prefix(&self, guild_id: Option<Id<GuildMarker>>) -> String {
        let lock = self.config.lock().unwrap();

        guild_id.map_or_else(
            || lock.prefix.to_string(),
            |guild_id| {
                lock.guild(guild_id)
                    .map_or_else(|| lock.prefix.to_string(), |data| data.prefix.to_string())
            },
        )
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
    let commands = Arc::new(commands_v2::bot::create_commands()?.build());

    let ctx = Context {
        config,
        http,
        cluster,
        application,
        user,
        cache,
        standby,
        commands,
        shard: None,
    };

    // TODO: To be implemented.
    // Spawn community event handler.
    // tokio::spawn(handle_timer(ctx.clone(), 60 * 5));

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
        println!("Event error: {e:#?}");
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

async fn handle_ready(ctx: &Context, ready: Ready) -> AnyResult<()> {
    println!("Ready: '{}'", ready.user.name);
    info!("Ready: '{}'", ready.user.name);

    // TODO: Get commands from `ctx`.
    let commands: Vec<_> = commands_v2::bot::create_commands()
        .unwrap()
        .build()
        .values()
        .flat_map(|c| c.twilight_commands().unwrap())
        .collect();

    debug!("Creating {} global commands", commands.len());

    // Set global application commands.
    ctx.http
        .interaction(ctx.application.id)
        .set_global_commands(&commands)
        .send()
        .await?;

    Ok(())
}

async fn handle_guild_create(ctx: &Context, guild: Guild) -> AnyResult<()> {
    println!("Guild: {}", guild.name);
    info!("Guild: '{}'", guild.name);

    let whitelist = ctx.config.lock().unwrap().whitelist.clone();

    // If whitelist is enabled, check if this guild is in it.
    if let Some(whitelist) = whitelist {
        if !whitelist.contains(&guild.id) {
            info!("Leaving a non-whitelisted guild '{}'", guild.id);
            ctx.http.leave_guild(guild.id).exec().await?;
        } else {
            debug!("Whitelisted guild: '{}'", guild.id)
        }
    }

    // ctx.http
    //     .interaction(ctx.application.id)
    //     .set_guild_commands(guild.id, &commands)
    //     .send()
    //     .await?;

    Ok(())
}

async fn handle_interaction_create(ctx: &Context, mut inter: Interaction) -> AnyResult<()> {
    // println!("{:#?}", inter);

    // Take interaction data from the interaction,
    // so that both can be passed forward without matching again.
    match inter.data.take() {
        Some(InteractionData::ApplicationCommand(d)) => {
            println!("{d:#?}");
            //
            let interaction = ctx.interaction();

            let resp = InteractionResponse {
                kind: InteractionResponseType::DeferredChannelMessageWithSource,
                data: Some(InteractionResponseData::default()),
            };

            // Acknowledge the interaction.
            interaction
                .create_response(inter.id, &inter.token, &resp)
                .exec()
                .await?;

            // TODO WIP

            // Delete the response.
            interaction.delete_response(&inter.token).exec().await?;

            // Process the command.
            let inter = Arc::new(inter); // Might be needed later.
            crate::commands_v2::handle::interaction_command(ctx, inter, Arc::new(*d)).await?;
        },
        Some(InteractionData::MessageComponent(d)) => {
            println!("{d:#?}");
            //
        },
        Some(InteractionData::ModalSubmit(d)) => {
            println!("{d:#?}");
            //
        },
        None => println!("{inter:#?}"),
    }

    Ok(())
}

async fn handle_message_create(ctx: &Context, msg: Message) -> AnyResult<()> {
    // Ignore bot users.
    if msg.author.bot {
        trace!("Message sender is a bot '{}'", msg.author.name);
        return Ok(());
    }

    let msg = Arc::new(msg);

    match crate::commands_v2::handle::classic_command(ctx, Arc::clone(&msg))
        .await
        .context("Failed to handle classic command")
    {
        Ok(k) => println!("{k:?}"),
        Err(e) if e.downcast_ref() == Some(&CommandError::NotPrefixed) => (), // Message was not a command.
        Err(e) => {
            // Log processing errors.
            eprintln!("Error processing command: {e}");
            error!("Error processing command: {e}");

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

    Ok(())
}

async fn handle_message_update(_ctx: &Context, _mu: MessageUpdate) -> AnyResult<()> {
    // TODO Check if updated message is something that should update content from the bot.

    // let text = mu.content.unwrap_or_default();

    // if let Some(author) = mu.author {
    //     if author.bot {
    //         return Ok(());
    //     }

    //     if let Some(ccall) = CommandCall::parse_from(ctx, mu.guild_id, &text) {
    //         println!("Updated message command: {ccall}");
    //     }
    // }

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

    // Remove reaction roles mappping, if deleted message was one.
    let key = config::reaction_roles_key(md.channel_id, md.id);
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
        // Calls single delete handler for every deletion.
        handle_message_delete(ctx, message_delete_with(id)).await?;
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

        let key = config::reaction_roles_key(reaction.channel_id, reaction.message_id);

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

        let key = config::reaction_roles_key(reaction.channel_id, reaction.message_id);

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
