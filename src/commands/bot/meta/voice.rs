use twilight_model::id::marker::GuildMarker;
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
        let guild_id = guild_id.ok_or_else(|| CommandError::Disabled)?;

        ctx.voice
            .join(guild_id, channel_id)
            .await
            .with_context(|| format!("Failed to join channel '{channel_id}'"))
            .map(|_| {
                Ok(Response::new(move || async move {
                    info!("Connected to voice channel '{channel_id}'");
                    Ok(())
                }))
            })?
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
        let guild_id = guild_id.ok_or_else(|| CommandError::Disabled)?;

        let channel_id = match ctx.voice.get(guild_id) {
            Some(call) => match call.lock().await.current_channel() {
                Some(channel_id) => channel_id,
                None => return Ok(Response::none()),
            },
            None => return Ok(Response::none()),
        };

        ctx.voice
            .leave(guild_id)
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
