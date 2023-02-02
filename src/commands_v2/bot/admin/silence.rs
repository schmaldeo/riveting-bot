use twilight_model::id::marker::{GuildMarker, UserMarker};
use twilight_model::id::Id;

use crate::commands_v2::prelude::*;
use crate::utils::prelude::*;

const DEFAULT_MUTE: u64 = 60;

pub struct Mute {
    guild_id: Option<Id<GuildMarker>>,
    user_id: Option<Id<UserMarker>>,
    duration: Option<u64>,
}

impl Mute {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands_v2::builder::*;

        command("mute", "Silence someone in voice channel.")
            .attach(Self::classic)
            .attach(Self::slash)
            .attach(Self::user)
            .permissions(Permissions::ADMINISTRATOR)
            .option(user("user", "Who to mute.").required())
            .option(integer("seconds", "Duration of the mute.").min(0))
    }

    pub async fn uber(self, ctx: Context) -> CommandResult {
        let Some(guild_id) = self.guild_id else {
            return Err(CommandError::Disabled)
        };

        let Some(user_id) = self.user_id else {
            return Err(CommandError::MissingArgs)
        };

        let timeout = self.duration.unwrap_or(DEFAULT_MUTE);

        ctx.http
            .update_guild_member(guild_id, user_id)
            .mute(true)
            .await?;

        tokio::time::sleep(std::time::Duration::from_secs(timeout)).await;

        // FIXME: This fails if the target user is not connected to a voice channel leaving them server muted.
        ctx.http
            .update_guild_member(guild_id, user_id)
            .mute(false)
            .await?;

        Ok(Response::Clear)
    }

    pub async fn classic(ctx: Context, req: ClassicRequest) -> CommandResult {
        Self {
            guild_id: req.message.guild_id,
            user_id: req.args.get("user").user().map(|r| r.id()),
            duration: req.args.get("duration").integer().map(|i| i as u64),
        }
        .uber(ctx)
        .await
    }

    pub async fn slash(ctx: Context, req: SlashRequest) -> CommandResult {
        Self {
            guild_id: req.interaction.guild_id,
            user_id: req.args.get("user").user().map(|r| r.id()),
            duration: req.args.get("duration").integer().map(|i| i as u64),
        }
        .uber(ctx)
        .await
    }

    pub async fn user(ctx: Context, req: UserRequest) -> CommandResult {
        Self {
            guild_id: req.interaction.guild_id,
            user_id: Some(req.target_id),
            duration: None, // TODO: Create modal for duration input.
        }
        .uber(ctx)
        .await
    }
}
