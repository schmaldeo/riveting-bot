use twilight_model::id::marker::{GuildMarker, UserMarker};
use twilight_model::id::Id;

use crate::commands_v2::prelude::*;
use crate::utils::prelude::*;

const DEFAULT_MUTE: u64 = 60;

#[derive(Default)]
pub struct Mute {
    guild_id: Option<Id<GuildMarker>>,
    user_id: Option<Id<UserMarker>>,
    duration: Option<u64>,
}

impl Command for Mute {
    type Data = Self;

    async fn uber(ctx: Context, data: Self::Data) -> CommandResult {
        let Some(guild_id) = data.guild_id else {
            return Err(CommandError::Disabled)
        };

        let Some(user_id) = data.user_id else {
            return Err(CommandError::MissingArgs)
        };

        let timeout = data.duration.unwrap_or(DEFAULT_MUTE);

        ctx.http
            .update_guild_member(guild_id, user_id)
            .mute(true)
            .await?;

        tokio::time::sleep(std::time::Duration::from_secs(timeout)).await;

        // TODO: This might fail if the target user is not connected to a voice channel.
        ctx.http
            .update_guild_member(guild_id, user_id)
            .mute(false)
            .await?;

        Ok(Response::Clear)
    }

    async fn classic(ctx: Context, req: ClassicRequest) -> CommandResult {
        Self::uber(ctx, Self {
            guild_id: req.message.guild_id,
            user_id: req.args.get("user").user().map(|r| r.id()),
            duration: req.args.get("duration").integer().map(|i| i as u64),
        })
        .await
    }

    async fn slash(ctx: Context, req: SlashRequest) -> CommandResult {
        Self::uber(ctx, Self {
            guild_id: req.interaction.guild_id,
            user_id: req.args.get("user").user().map(|r| r.id()),
            duration: req.args.get("duration").integer().map(|i| i as u64),
        })
        .await
    }

    async fn user(ctx: Context, req: UserRequest) -> CommandResult {
        Self::uber(ctx, Self {
            guild_id: req.interaction.guild_id,
            user_id: req.data.resolved.as_ref().and_then(|d| {
                d.users
                    .iter()
                    .filter(|(id, _)| **id != req.interaction.author_id().unwrap())
                    .map(|(id, _)| id)
                    .next()
                    .cloned()
            }),
            duration: None, // TODO: Create modal for duration input.
        })
        .await
    }
}
