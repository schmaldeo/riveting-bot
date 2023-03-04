#![feature(associated_type_bounds)]
#![feature(associated_type_defaults)]
#![feature(decl_macro)]
#![feature(iter_intersperse)]
#![feature(iterator_try_collect)]
#![feature(iterator_try_reduce)]
#![feature(option_get_or_insert_default)]
#![feature(pattern)]
#![feature(trait_alias)]
//
#![allow(dead_code)]
#![allow(clippy::significant_drop_in_scrutinee)]

use std::sync::{Arc, Mutex};
use std::{env, fs};

use tokio::runtime::Runtime;
use tracing::Level;
use tracing_subscriber::EnvFilter;
use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::{Cluster, Event};
use twilight_http::client::InteractionClient;
use twilight_http::Client;
use twilight_model::application::command::permissions::GuildCommandPermissions;
use twilight_model::application::interaction::{Interaction, InteractionData};
use twilight_model::channel::{Channel, Message};
use twilight_model::gateway::event::shard::Connected;
use twilight_model::gateway::payload::incoming::{
    ChannelUpdate, MessageDelete, MessageDeleteBulk, MessageUpdate, Ready, RoleUpdate,
};
use twilight_model::gateway::{GatewayReaction, Intents};
use twilight_model::guild::{Guild, Role};
use twilight_model::id::marker::{ChannelMarker, GuildMarker, RoleMarker};
use twilight_model::id::Id;
use twilight_model::oauth::Application;
use twilight_model::user::CurrentUser;
use twilight_model::voice::VoiceState;
use twilight_standby::Standby;

use crate::commands::{CommandError, Commands};
use crate::config::Config;
use crate::utils::prelude::*;

mod commands;

// mod commands;
mod config;
mod parser;
mod utils;

#[derive(Debug, Clone)]
pub struct Context {
    /// Bot configuration.
    config: Arc<Mutex<Config>>,
    /// Bot commands list.
    commands: Arc<Commands>,
    /// Application http client.
    http: Arc<Client>,
    /// Application shard manager.
    cluster: Arc<Cluster>,
    /// Application information.
    application: Arc<Application>,
    /// Application bot user.
    user: Arc<CurrentUser>,
    /// Caching of events.
    cache: Arc<InMemoryCache>,
    /// Standby event system.
    standby: Arc<Standby>,
    /// Async runtime.
    runtime: Arc<Runtime>,
    /// Shard id associated with the event.
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

    /// Get role objects with `ids` from cache or fetch from client.
    pub async fn roles_from(
        &self,
        guild_id: Id<GuildMarker>,
        ids: &[Id<RoleMarker>],
    ) -> AnyResult<Vec<Role>> {
        // Try to get the roles from cache.
        let cached_roles = ids
            .iter()
            .map(|id| self.cache.role(*id).map(|r| r.resource().to_owned()))
            .try_collect();

        // Use cached roles or otherwise fetch from client.
        match cached_roles {
            Some(r) => Ok(r),
            None => self.fetch_roles_from(guild_id, ids).await,
        }
    }

    /// Fetch role objects with `ids` from client without cache.
    pub async fn fetch_roles_from(
        &self,
        guild_id: Id<GuildMarker>,
        ids: &[Id<RoleMarker>],
    ) -> AnyResult<Vec<Role>> {
        let mut fetch = self.http.roles(guild_id).send().await?;

        // Manually update the cache.
        for role in fetch.iter().cloned() {
            self.cache.update(&RoleUpdate { guild_id, role });
        }

        fetch.retain(|r| ids.contains(&r.id));

        Ok(fetch)
    }

    /// Get the channel object from cache or fetch from client.
    pub async fn channel_from(&self, channel_id: Id<ChannelMarker>) -> AnyResult<Channel> {
        match self.cache.channel(channel_id) {
            Some(chan) => {
                // Use cached channel.
                Ok(chan.to_owned())
            },
            None => {
                // Fetch channel from the http client.
                let chan = self.http.channel(channel_id).send().await?;

                // Manually update the cache.
                self.cache.update(&ChannelUpdate(chan.clone()));

                Ok(chan)
            },
        }
    }
}

#[tracing::instrument]
fn main() -> AnyResult<()> {
    let rt = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?,
    );
    rt.block_on(async_main(Arc::clone(&rt)))
}

async fn async_main(runtime: Arc<Runtime>) -> AnyResult<()> {
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
    let commands = Arc::new(commands::bot::create_commands()?);

    let ctx = Context {
        config,
        commands,
        http,
        cluster,
        application,
        user,
        cache,
        standby,
        runtime,
        shard: None,
    };

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
#[tracing::instrument(name = "events", skip_all, fields(event = event.kind().name()))]
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
        Event::CommandPermissionsUpdate(cpu) => {
            handle_command_permissions_update(&ctx, cpu.0).await
        },

        // Gateway events.
        Event::GatewayHeartbeat(_)
        | Event::GatewayHeartbeatAck
        | Event::GatewayHello(_)
        | Event::GatewayInvalidateSession(_)
        | Event::GatewayReconnect => {
            debug!("Gateway event: {:?}", event.kind());
            Ok(())
        },

        // Other events here...
        event => {
            println!("Event: {:?}", event.kind());
            debug!("Event: {:?}", event.kind());
            Ok(())
        },
    };

    if let Err(e) = result {
        let chain = e.oneliner();
        eprintln!("Event error: {e:?}");
        error!("Event error: {chain}");

        if let Ok(id) = env::var("DISCORD_BOTDEV_CHANNEL") {
            // Send error as message on bot dev channel.
            let bot_dev = Id::new(id.parse()?);
            ctx.http
                .create_message(bot_dev)
                .content(&format!("{e:?}"))?
                .send()
                .await?;
        }
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

    let commands = ctx.commands.twilight_commands()?;

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
            ctx.http.leave_guild(guild.id).await?;
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
            crate::commands::handle::application_command(ctx, inter, *d)
                .await
                .context("Failed to handle application command")?;
        },
        Some(InteractionData::MessageComponent(d)) => {
            println!("{d:#?}");
            //
        },
        Some(InteractionData::ModalSubmit(d)) => {
            println!("{d:#?}");
            //
        },
        Some(d) => {
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

    #[cfg(feature = "ban-at-everyone")]
    check_if_at_everyone(ctx, &msg).await.ok();

    match crate::commands::handle::classic_command(ctx, Arc::clone(&msg)).await {
        Err(CommandError::NotPrefixed) => {
            // Message was not a command.

            if msg.mentions.iter().any(|mention| mention.id == ctx.user.id) {
                // Send bot help message.
                let about_msg = format!(
                    "Try `/about` or `{prefix}about` for general info, or `/help` or \
                     `{prefix}help` for commands.",
                    prefix = ctx.classic_prefix(msg.guild_id),
                );

                ctx.http
                    .create_message(msg.channel_id)
                    .content(&about_msg)?
                    .reply(msg.id)
                    .await?;
            }
            Ok(())
        },
        Err(CommandError::AccessDenied) => {
            ctx.http
                .create_message(msg.channel_id)
                .content("Rekt, you cannot use that. :melting_face:")?
                .reply(msg.id)
                .await?;
            Ok(())
        },
        res => res.context("Failed to handle classic command"),
    }
}
#[cfg(feature = "ban-at-everyone")]
async fn check_if_at_everyone(ctx: &Context, msg: &Message) -> AnyResult<()> {
    let Some(guild_id) = msg.guild_id else {
        anyhow::bail!("Disabled");
    };

    if msg.mention_everyone {
        ctx.http.create_ban(guild_id, msg.author.id).await?;
    }

    Ok(())
}

async fn handle_message_update(_ctx: &Context, _mu: MessageUpdate) -> AnyResult<()> {
    // TODO Check if updated message is something that should update content from the bot.

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

async fn handle_reaction_add(ctx: &Context, reaction: GatewayReaction) -> AnyResult<()> {
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
            .await?;
    }

    Ok(())
}

async fn handle_reaction_remove(ctx: &Context, reaction: GatewayReaction) -> AnyResult<()> {
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
            .await?;
    }

    Ok(())
}

async fn handle_voice_state(_ctx: &Context, voice: VoiceState) -> AnyResult<()> {
    println!("{voice:#?}",);

    Ok(())
}

async fn handle_command_permissions_update(
    _ctx: &Context,
    cpu: GuildCommandPermissions,
) -> AnyResult<()> {
    println!("Permissions update: {:#?}", cpu);
    // cpu.permissions.into_iter().for_each(|p| match p.id {
    //     CommandPermissionType::Channel(_) => todo!(),
    //     CommandPermissionType::Role(_) => todo!(),
    //     CommandPermissionType::User(_) => todo!(),
    // });
    Ok(())
}

fn intents() -> Intents {
    #[cfg(feature = "all-intents")]
    {
        Intents::all()
    }

    #[cfg(not(feature = "all-intents"))]
    {
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
