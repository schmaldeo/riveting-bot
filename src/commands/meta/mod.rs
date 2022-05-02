use indoc::formatdoc;

use crate::commands::{CommandContext, CommandResult};
use crate::utils::prelude::*;

/// Command: Ping Pong!
pub async fn ping(cc: CommandContext<'_>) -> CommandResult {
    cc.http
        .create_message(cc.msg.channel_id)
        .reply(cc.msg.id)
        .content("Pong!")?
        .send()
        .await?;
    Ok(())
}

/// Command: Info about the bot.
pub async fn about(cc: CommandContext<'_>) -> CommandResult {
    let about_msg = formatdoc!(
        "
        I am a RivetingBot, my source is available at <{link}>.
        You can list my commands with the `{prefix}help` command.
        My current version *(allegedly)* is `{version}`.
        ",
        link = env!("CARGO_PKG_REPOSITORY"),
        prefix = cc.active_prefix(cc.msg.guild_id),
        version = env!("CARGO_PKG_VERSION")
    );

    cc.http
        .create_message(cc.msg.channel_id)
        .reply(cc.msg.id)
        .content(&about_msg)?
        .send()
        .await?;

    Ok(())
}

/// Command: Help for using the bot, commands and usage.
pub async fn help(cc: CommandContext<'_>) -> CommandResult {
    let help_msg = {
        let lock = cc.config.lock().unwrap();
        let global_prefix = &lock.global.prefix;
        let mut prefix_msg = format!("Prefix: '{}'", global_prefix);

        if let Some(guild_id) = cc.msg.guild_id {
            if let Some(data) = lock.guilds.get(&guild_id) {
                prefix_msg = formatdoc!(
                    "
                    Default prefix: '{}'
                    Guild prefix: '{}'",
                    global_prefix,
                    data.prefix
                );
            }
        }

        formatdoc!(
            "```yaml
            {}
            Commands:
            {}
            ```",
            prefix_msg,
            cc.chat_commands
        )
    };

    cc.http
        .create_message(cc.msg.channel_id)
        .reply(cc.msg.id)
        .content(&help_msg)?
        .send()
        .await?;
    Ok(())
}
