use indoc::formatdoc;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, MessageMarker};
use twilight_model::id::Id;

use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;
use crate::Context;

/// Command: Ping Pong!
pub struct Ping;

impl Ping {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands_v2::builder::*;

        command("ping", "Ping the bot.")
            .attach(Self::classic)
            .attach(Self::slash)
            .dm()
    }

    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        Ok(Response::CreateMessage("Pong!".to_string()))
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        Ok(Response::CreateMessage("Pong!".to_string()))
    }
}

/// Command: Info about the bot.
pub struct About {
    guild_id: Option<Id<GuildMarker>>,
    channel_id: Option<Id<ChannelMarker>>,
    message_id: Option<Id<MessageMarker>>,
}

impl About {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands_v2::builder::*;

        command("about", "Display info about the bot.")
            .attach(Self::classic)
            .attach(Self::slash)
            .dm()
    }

    async fn uber(self, ctx: Context) -> CommandResult {
        let about_msg = formatdoc!(
            "I am a RivetingBot!
            You can list my commands with `/help` or `{prefix}help` command.
            My current version *(allegedly)* is `{version}`.
            My source is available at <{link}>
            ",
            prefix = ctx.classic_prefix(self.guild_id),
            version = env!("CARGO_PKG_VERSION"),
            link = env!("CARGO_PKG_REPOSITORY"),
        );

        Ok(Response::CreateMessage(about_msg))
    }

    async fn classic(ctx: Context, req: ClassicRequest) -> CommandResult {
        Self {
            guild_id: req.message.guild_id,
            channel_id: Some(req.message.channel_id),
            message_id: Some(req.message.id),
        }
        .uber(ctx)
        .await
    }

    async fn slash(ctx: Context, req: SlashRequest) -> CommandResult {
        Self {
            guild_id: req.interaction.guild_id,
            channel_id: req.interaction.channel_id,
            message_id: None,
        }
        .uber(ctx)
        .await
    }
}

/// Command: Help for using the bot, commands and usage.
pub struct Help {
    args: Args,
    guild_id: Option<Id<GuildMarker>>,
    channel_id: Option<Id<ChannelMarker>>,
    message_id: Option<Id<MessageMarker>>,
}

impl Help {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands_v2::builder::*;

        command("help", "List bot commands.")
            .attach(Self::classic)
            .attach(Self::slash)
            .option(string("command", "Get help on a command."))
            .dm()
    }

    async fn uber(self, ctx: Context) -> CommandResult {
        if let Ok(_value) = self.args.string("command") {
            // TODO: If "command" argument exists, show help on that command instead.
            todo!("get rekt");
        }

        let help_msg = {
            formatdoc!(
                "```yaml
                Prefix: '/' or '{prefix}'
                Commands:
                {commands}
                ```",
                prefix = ctx.classic_prefix(self.guild_id),
                commands = ctx.commands
            )
        };

        Ok(Response::CreateMessage(help_msg))
    }

    async fn classic(ctx: Context, req: ClassicRequest) -> CommandResult {
        Self {
            args: req.args,
            guild_id: req.message.guild_id,
            channel_id: Some(req.message.channel_id),
            message_id: Some(req.message.id),
        }
        .uber(ctx)
        .await
    }

    async fn slash(ctx: Context, req: SlashRequest) -> CommandResult {
        Self {
            args: req.args,
            guild_id: req.interaction.guild_id,
            channel_id: req.interaction.channel_id,
            message_id: None,
        }
        .uber(ctx)
        .await
    }
}
