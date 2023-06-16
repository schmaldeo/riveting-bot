use songbird::input::{Input, YoutubeDl};
use songbird::tracks::Track;
use twilight_model::channel::ChannelType;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, UserMarker};
use twilight_model::id::Id;

use crate::commands::prelude::*;
use crate::utils::prelude::*;

/// Command: Voice channel controls.
pub struct Voice;

impl Voice {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands::builder::*;

        command("voice", "Manage voice connection.")
            .attach(Self::classic)
            .attach(Self::slash)
            .option(
                sub("join", "Join the bot to a voice channel.")
                    .attach(Join::classic)
                    .attach(Join::slash)
                    .option(
                        channel("channel", "Voice channel to join.")
                            .types([ChannelType::GuildVoice, ChannelType::GuildStageVoice]),
                    ),
            )
            .option(
                sub("leave", "Disconnect the bot from a voice channel.")
                    .attach(Leave::classic)
                    .attach(Leave::slash),
            )
            .option(
                sub("play", "Play a sound or music on voice (queued).")
                    .attach(Play::classic)
                    .attach(Play::slash)
                    .option(string("url", "Youtube URL to play.").required()),
            )
            .option(
                sub("skip", "Go to the next track in queue.")
                    .attach(Skip::classic)
                    .attach(Skip::slash),
            )
    }

    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResponse {
        todo!();
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResponse {
        todo!();
    }
}

/// Command: Tell the bot to connect to a voice channel.
struct Join;

impl Join {
    async fn uber(
        ctx: Context,
        args: Args,
        guild_id: Option<Id<GuildMarker>>,
        req_channel_id: Id<ChannelMarker>,
        user_id: Id<UserMarker>,
    ) -> CommandResponse {
        let guild_id = guild_id.ok_or_else(|| CommandError::Disabled)?;
        // If no arg was given, try to find user in voice channels, otherwise use channel id from the request itself.
        let channel_id = match args.channel("channel") {
            Ok(c) => c.id(),
            Err(e) => {
                debug!("{e}; Using fallback");
                match ctx.user_voice_channel(guild_id, user_id).await {
                    Ok(channel_id) => {
                        debug!("User '{user_id}' was found in voice channel '{channel_id}'");
                        channel_id
                    },
                    Err(e) => match ctx.channel_from(req_channel_id).await?.kind {
                        ChannelType::GuildVoice | ChannelType::GuildStageVoice => {
                            debug!("{e}; Using request channel");
                            req_channel_id
                        },
                        _ => return Err(e.into()),
                    },
                }
            },
        };

        ctx.voice
            .join(guild_id, channel_id)
            .await
            .with_context(|| format!("Failed to join channel '{channel_id}'"))
            .map(|c| {
                Ok(Response::new(move || async move {
                    let deaf = c.lock().await.deafen(true).await;
                    deaf.context("Failed to deafen")?;
                    info!("Connected to voice channel '{channel_id}'");
                    Ok(())
                }))
            })?
    }

    async fn classic(ctx: Context, req: ClassicRequest) -> CommandResponse {
        Self::uber(
            ctx,
            req.args,
            req.message.guild_id,
            req.message.channel_id,
            req.message.author.id,
        )
        .await
    }

    async fn slash(ctx: Context, req: SlashRequest) -> CommandResponse {
        Self::uber(
            ctx,
            req.args,
            req.interaction.guild_id,
            req.interaction
                .channel
                .as_ref()
                .map(|c| c.id)
                .context("No channel found")?,
            req.interaction.author_id().context("No user id found")?,
        )
        .await
    }
}

/// Command: Tell the bot to disconnect from a voice channel.
struct Leave;

impl Leave {
    async fn uber(ctx: Context, guild_id: Option<Id<GuildMarker>>) -> CommandResponse {
        let guild_id = guild_id.ok_or_else(|| CommandError::Disabled)?;

        let channel_id = match ctx.voice.get(guild_id) {
            Some(call) => match call.lock().await.current_channel() {
                Some(channel_id) => channel_id,
                None => return Ok(Response::none()),
            },
            None => return Ok(Response::none()),
        };

        ctx.voice
            .remove(guild_id)
            .await
            .with_context(|| format!("Failed to leave channel '{channel_id}'"))
            .map(|_| {
                Ok(Response::new(move || async move {
                    info!("Disconnected from voice channel '{channel_id}'");
                    Ok(())
                }))
            })?
    }

    async fn classic(ctx: Context, req: ClassicRequest) -> CommandResponse {
        Self::uber(ctx, req.message.guild_id).await
    }

    async fn slash(ctx: Context, req: SlashRequest) -> CommandResponse {
        Self::uber(ctx, req.interaction.guild_id).await
    }
}

/// Command: Play a sound or music in voice.
struct Play;

impl Play {
    async fn uber(
        ctx: Context,
        args: Args,
        guild_id: Id<GuildMarker>,
        channel_id: Id<ChannelMarker>,
    ) -> CommandResponse {
        let Some(call) = ctx.voice.get(guild_id) else {
            info!("No voice connection found for '{guild_id}'");
            return Ok(Response::none());
        };

        let url = args.string("url")?;
        let client = reqwest::Client::new();
        let mut input = Input::from(YoutubeDl::new(client, url.into_string()));
        let meta = input.aux_metadata().await;
        let track = Track::new(input).volume(0.5);
        let handle = call.lock().await.enqueue(track).await;
        let result = handle
            .make_playable_async()
            .await
            .with_context(|| format!("Cannot play audio track"));

        match meta {
            Ok(metadata) => {
                let content = format!(
                    "Playing **{}** by **{}**",
                    metadata
                        .title
                        .or(metadata.track)
                        .unwrap_or_else(|| "<UNKNOWN>".to_string()),
                    metadata.artist.unwrap_or_else(|| "<UNKNOWN>".to_string()),
                );

                ctx.http
                    .create_message(channel_id)
                    .content(&content)?
                    .await?;

                // Metadata was ok, but playing failed?
                result?;
            },
            Err(e) => {
                eprintln!("Metadata error: {e}");
                info!("Metadata error: {e}");

                if let Err(r) = result {
                    eprintln!("Cannot play audio track: {r}");
                    info!("Cannot play audio track: {r}");

                    ctx.http
                        .create_message(channel_id)
                        .content(&format!("I can't play that :mute:"))?
                        .await?;
                } else {
                    info!("Something without metadata is playing");
                }
            },
        }

        Ok(Response::none())
    }

    async fn classic(ctx: Context, req: ClassicRequest) -> CommandResponse {
        Self::uber(
            ctx,
            req.args,
            req.message.guild_id.ok_or(CommandError::Disabled)?,
            req.message.channel_id,
        )
        .await
    }

    async fn slash(ctx: Context, req: SlashRequest) -> CommandResponse {
        Self::uber(
            ctx,
            req.args,
            req.interaction.guild_id.ok_or(CommandError::Disabled)?,
            req.interaction
                .channel
                .as_ref()
                .map(|c| c.id)
                .context("No channel found")?,
        )
        .await
    }
}

/// Command: Go to the next track in queue.
struct Skip;

impl Skip {
    async fn uber(ctx: Context, guild_id: Id<GuildMarker>) -> CommandResponse {
        let Some(call) = ctx.voice.get(guild_id) else {
            info!("No voice connection found for '{guild_id}'");
            return Ok(Response::none());
        };

        call.lock()
            .await
            .queue()
            .skip()
            .context("Failed to skip audio track")?;

        Ok(Response::none())
    }

    async fn classic(ctx: Context, req: ClassicRequest) -> CommandResponse {
        Self::uber(ctx, req.message.guild_id.ok_or(CommandError::Disabled)?).await
    }

    async fn slash(ctx: Context, req: SlashRequest) -> CommandResponse {
        Self::uber(ctx, req.interaction.guild_id.ok_or(CommandError::Disabled)?).await
    }
}
