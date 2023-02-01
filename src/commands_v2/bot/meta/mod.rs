use indoc::formatdoc;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, MessageMarker};
use twilight_model::id::Id;

use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;
use crate::Context;

/// Command: Ping Pong!
#[derive(Default)]
pub struct Ping;

impl Command for Ping {
    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        Ok(Response::CreateMessage("Pong!".to_string()))
    }
}

/// Command: Info about the bot.
#[derive(Default)]
pub struct About {
    guild_id: Option<Id<GuildMarker>>,
    channel_id: Option<Id<ChannelMarker>>,
    message_id: Option<Id<MessageMarker>>,
}

impl Command for About {
    type Data = Self;

    async fn uber(ctx: Context, data: Self::Data) -> CommandResult {
        let about_msg = formatdoc!(
            "I am a RivetingBot!
            You can list my commands with `/help` or `{prefix}help` command.
            My current version *(allegedly)* is `{version}`.
            My source is available at <{link}>
            ",
            prefix = ctx.classic_prefix(data.guild_id),
            version = env!("CARGO_PKG_VERSION"),
            link = env!("CARGO_PKG_REPOSITORY"),
        );

        Ok(Response::CreateMessage(about_msg))
    }

    async fn classic(ctx: Context, req: ClassicRequest) -> CommandResult {
        Self::uber(ctx, Self {
            guild_id: req.message.guild_id,
            channel_id: Some(req.message.channel_id),
            message_id: Some(req.message.id),
        })
        .await
    }

    async fn slash(ctx: Context, req: SlashRequest) -> CommandResult {
        Self::uber(ctx, Self {
            guild_id: req.interaction.guild_id,
            channel_id: req.interaction.channel_id,
            message_id: None,
        })
        .await
    }
}

/// Command: Help for using the bot, commands and usage.
#[derive(Default)]
pub struct Help {
    args: Args,
    guild_id: Option<Id<GuildMarker>>,
    channel_id: Option<Id<ChannelMarker>>,
    message_id: Option<Id<MessageMarker>>,
}

impl Command for Help {
    type Data = Self;

    async fn uber(ctx: Context, data: Self::Data) -> CommandResult {
        if let Some(_value) = data.args.get("command").string() {
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
                prefix = ctx.classic_prefix(data.guild_id),
                commands = ctx.commands
            )
        };

        Ok(Response::CreateMessage(help_msg))
    }

    async fn classic(ctx: Context, req: ClassicRequest) -> CommandResult {
        Self::uber(ctx, Self {
            args: req.args,
            guild_id: req.message.guild_id,
            channel_id: Some(req.message.channel_id),
            message_id: Some(req.message.id),
        })
        .await
    }

    async fn slash(ctx: Context, req: SlashRequest) -> CommandResult {
        Self::uber(ctx, Self {
            args: req.args,
            guild_id: req.interaction.guild_id,
            channel_id: req.interaction.channel_id,
            message_id: None,
        })
        .await
    }
}
