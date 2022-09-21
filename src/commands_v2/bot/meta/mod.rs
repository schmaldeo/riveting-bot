use indoc::formatdoc;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, MessageMarker};
use twilight_model::id::Id;

use crate::commands_v2::prelude::*;
use crate::utils::prelude::*;
use crate::Context;

pub struct Ping;

impl Command for Ping {
    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        Ok(Response::CreateMessage("Pong!".to_string()))
    }
}

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
            You can list my commands with the `{prefix}help` command.
            My current version *(allegedly)* is `{version}`.
            My source is available at <{link}>
            ",
            prefix = ctx.classic_prefix(data.guild_id),
            version = env!("CARGO_PKG_VERSION"),
            link = env!("CARGO_PKG_REPOSITORY"),
        );

        let res = ctx
            .http
            .create_message(data.channel_id.unwrap())
            .content(&about_msg)?;

        match data.message_id {
            Some(message_id) => res.reply(message_id),
            None => res,
        }
        .send()
        .await?;

        Ok(Response::Clear)
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

#[derive(Default)]
pub struct Help {
    guild_id: Option<Id<GuildMarker>>,
    channel_id: Option<Id<ChannelMarker>>,
    message_id: Option<Id<MessageMarker>>,
}

impl Command for Help {
    type Data = Self;

    async fn uber(ctx: Context, data: Self::Data) -> CommandResult {
        let help_msg = {
            let lock = ctx.config.lock().unwrap();
            let global_prefix = &lock.prefix;
            let mut prefix_msg = format!("Prefix: '{global_prefix}'");
            if let Some(guild_id) = data.guild_id {
                if let Some(data) = lock.guild(guild_id) {
                    prefix_msg = formatdoc!(
                        "
                        Default prefix: '{}'
                        Guild prefix: '{}'
                        ",
                        global_prefix,
                        data.prefix
                    );
                }
            }
            // FIXME: Commands v2 display
            formatdoc!(
                "```yaml
                {}
                Commands:
                {:?}
                ```",
                prefix_msg,
                ctx.commands
            )
        };

        let res = ctx
            .http
            .create_message(data.channel_id.unwrap())
            .content(&help_msg)?;

        match data.message_id {
            Some(message_id) => res.reply(message_id),
            None => res,
        }
        .send()
        .await?;

        Ok(Response::Clear)
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
