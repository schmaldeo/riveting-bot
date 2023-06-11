use std::sync::atomic::{AtomicU64, Ordering};

use twilight_model::gateway::payload::outgoing::UpdateVoiceState;
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;

use crate::commands::prelude::*;
use crate::utils::prelude::*;

/// Connected voice channel id, `0` if not connected.
static CONNECTED: AtomicU64 = AtomicU64::new(0);

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
                            .required()
                            .types([ChannelType::GuildVoice, ChannelType::GuildStageVoice]),
                    ),
            )
            .option(
                sub("leave", "Disconnect the bot from a voice channel.")
                    .attach(Leave::classic)
                    .attach(Leave::slash),
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
    async fn uber(ctx: Context, args: Args, guild_id: Option<Id<GuildMarker>>) -> CommandResponse {
        // TODO: This could also be inferred from user's voice channel or voice channel chat.
        let channel_id = args.channel("channel")?.id();
        ctx.shard
            .context("No associated shard")?
            .sender
            .command(&UpdateVoiceState::new(
                guild_id.ok_or_else(|| CommandError::Disabled)?,
                Some(channel_id),
                false,
                false,
            ))
            .map_or_else(
                |e| Err(e.into()),
                move |_| {
                    Ok(Response::new(move || async move {
                        let old = CONNECTED.swap(channel_id.get(), Ordering::Relaxed);
                        info!(old, "Connected to voice channel '{channel_id}'");
                        Ok(())
                    }))
                },
            )
    }

    async fn classic(ctx: Context, req: ClassicRequest) -> CommandResponse {
        Self::uber(ctx, req.args, req.message.guild_id).await
    }

    async fn slash(ctx: Context, req: SlashRequest) -> CommandResponse {
        Self::uber(ctx, req.args, req.interaction.guild_id).await
    }
}

/// Command: Tell the bot to disconnect from a voice channel.
struct Leave;

impl Leave {
    async fn uber(ctx: Context, guild_id: Option<Id<GuildMarker>>) -> CommandResponse {
        ctx.shard
            .context("No associated shard")?
            .sender
            .command(&UpdateVoiceState::new(
                guild_id.ok_or_else(|| CommandError::Disabled)?,
                None,
                false,
                false,
            ))
            .map_or_else(
                |e| Err(e.into()),
                move |_| {
                    Ok(Response::new(move || async move {
                        let old = CONNECTED.swap(0, Ordering::Relaxed);
                        info!("Disconnected from voice channel '{old}'");
                        Ok(())
                    }))
                },
            )
    }

    async fn classic(ctx: Context, req: ClassicRequest) -> CommandResponse {
        Self::uber(ctx, req.message.guild_id).await
    }

    async fn slash(ctx: Context, req: SlashRequest) -> CommandResponse {
        Self::uber(ctx, req.interaction.guild_id).await
    }
}
