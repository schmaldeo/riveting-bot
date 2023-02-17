use twilight_model::id::marker::{ChannelMarker, GuildMarker};
use twilight_model::id::Id;

use crate::commands::prelude::*;
use crate::utils::prelude::*;

/// Command: Create or edit bot messages.
pub struct Bot;

impl Bot {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands::builder::*;

        command("bot", "Create or edit bot messages.")
            .attach(Self::classic)
            .attach(Self::slash)
            .permissions(Permissions::ADMINISTRATOR)
            .option(
                sub("say", "Post a message by the bot.")
                    .attach(Say::classic)
                    .attach(Say::slash)
                    .option(string("text", "What to say.").required()),
            )
            .option(
                sub("edit", "Edit an existing bot message.")
                    .attach(Edit::classic)
                    .option(message("message", "Message to edit.").required())
                    .option(string("text", "New content.").required()),
            )
    }

    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Post a message as the bot.
struct Say {
    guild_id: Option<Id<GuildMarker>>,
    channel_id: Id<ChannelMarker>,
    args: Args,
}

impl Say {
    async fn uber(self, ctx: Context) -> CommandResult {
        if self.guild_id.is_none() {
            return Err(CommandError::Disabled);
        }

        let text = self.args.string("text")?;
        let empty = text.trim().is_empty();
        let content = if empty { "no u" } else { &text };

        let msg = ctx
            .http
            .create_message(self.channel_id)
            .content(content)?
            .send()
            .await?;

        info!("Bot message created with id '{}'", msg.id);

        Ok(Response::Clear)
    }

    async fn classic(ctx: Context, req: ClassicRequest) -> CommandResult {
        Self {
            guild_id: req.message.guild_id,
            channel_id: req.message.channel_id,
            args: req.args,
        }
        .uber(ctx)
        .await
    }

    async fn slash(ctx: Context, req: SlashRequest) -> CommandResult {
        let Some(channel_id) = req.interaction.channel_id else {
            return Err(CommandError::MissingArgs);
        };

        Self {
            guild_id: req.interaction.guild_id,
            channel_id,
            args: req.args,
        }
        .uber(ctx)
        .await
    }
}

/// Command: Edit a message created by the bot (can be anything).
struct Edit;

impl Edit {
    async fn classic(ctx: Context, req: ClassicRequest) -> CommandResult {
        if req.message.guild_id.is_none() {
            return Err(CommandError::Disabled);
        }

        let Some(replied) = &req.message.referenced_message else {
            return Err(CommandError::MissingReply);
        };

        // Ignore if replied message is not from this bot.
        if replied.author.id != ctx.user.id {
            return Err(CommandError::UnexpectedArgs(
                "Replied message is not from this bot".to_string(),
            ));
        }

        // let msg = req.args.message("message")?;
        let text = req.args.string("text")?;
        let empty = text.trim().is_empty();
        let content = if empty { "no u" } else { &text };

        ctx.http
            .update_message(replied.channel_id, replied.id)
            .content(Some(content))?
            .await?;

        info!("Bot message edited with id '{}'", replied.id);

        Ok(Response::Clear)
    }
}
