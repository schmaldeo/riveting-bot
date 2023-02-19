use twilight_gateway::Event;
use twilight_model::channel::ChannelType;
use twilight_model::id::marker::{GuildMarker, UserMarker};
use twilight_model::id::Id;

use crate::commands::prelude::*;
use crate::utils::prelude::*;

const DEFAULT_MUTE: u64 = 60;

/// Command: Silence a voice user for a set amount of time.
pub struct Mute {
    guild_id: Option<Id<GuildMarker>>,
    user_id: Id<UserMarker>,
    duration: Option<u64>,
}

impl Mute {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands::builder::*;

        command("mute", "Silence someone in voice channel.")
            .attach(Self::classic)
            .attach(Self::slash)
            .attach(Self::user)
            .permissions(Permissions::ADMINISTRATOR)
            .option(user("user", "Who to mute.").required())
            .option(integer("seconds", "Duration of the mute.").min(0))
    }

    async fn uber(self, ctx: Context) -> CommandResult {
        let Some(guild_id) = self.guild_id else {
            return Err(CommandError::Disabled)
        };

        let timeout = self.duration.unwrap_or(DEFAULT_MUTE);

        // This fails if the target user is not connected to a voice channel.
        if ctx
            .http
            .update_guild_member(guild_id, self.user_id)
            .mute(true)
            .await
            .is_err()
        {
            return Ok(Response::None); // Nothing more to do here.
        }

        tokio::time::sleep(std::time::Duration::from_secs(timeout)).await;

        let channels = ctx.http.guild_channels(guild_id).send().await?;
        let on_voice = channels
            .into_iter()
            .filter(|c| {
                matches!(
                    c.kind,
                    ChannelType::GuildVoice | ChannelType::GuildStageVoice
                )
            })
            .filter_map(|c| c.recipients)
            .flatten()
            .any(|u| u.id == self.user_id);

        if !on_voice {
            // Wait for user to connect to voice.
            ctx.standby
                .wait_for(guild_id, move |event: &Event| match event {
                    Event::VoiceStateUpdate(data) => data
                        .member
                        .as_ref()
                        .map_or(false, |m| m.user.id == self.user_id),
                    _ => false,
                })
                .await?;
        }

        // This fails if the target user is not connected to a voice channel leaving them server muted.
        ctx.http
            .update_guild_member(guild_id, self.user_id)
            .mute(false)
            .await?;

        Ok(Response::None) // Already cleared.
    }

    async fn classic(ctx: Context, req: ClassicRequest) -> CommandResult {
        req.clear(&ctx).await?;

        Self {
            guild_id: req.message.guild_id,
            user_id: req.args.user("user").map(|r| r.id())?,
            duration: req.args.integer("seconds").map(|i| i as u64).ok(),
        }
        .uber(ctx)
        .await
    }

    async fn slash(ctx: Context, req: SlashRequest) -> CommandResult {
        req.clear(&ctx).await?;

        Self {
            guild_id: req.interaction.guild_id,
            user_id: req.args.user("user").map(|r| r.id())?,
            duration: req.args.integer("seconds").map(|i| i as u64).ok(),
        }
        .uber(ctx)
        .await
    }

    async fn user(ctx: Context, req: UserRequest) -> CommandResult {
        req.clear(&ctx).await?;

        Self {
            guild_id: req.interaction.guild_id,
            user_id: req.target_id,
            duration: None, // TODO: Create modal for duration input.
        }
        .uber(ctx)
        .await
    }
}
