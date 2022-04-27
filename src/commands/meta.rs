use crate::commands::{CommandContext, CommandResult};
use crate::utils::*;

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
    let about_msg = "I am a RivetingBot";
    cc.http
        .create_message(cc.msg.channel_id)
        .reply(cc.msg.id)
        .content(about_msg)?
        .send()
        .await?;
    Ok(())
}

/// Command: Help for using the bot, commands and usage.
pub async fn help(cc: CommandContext<'_>) -> CommandResult {
    let help_msg = format!("{}", cc.chat_commands);
    cc.http
        .create_message(cc.msg.channel_id)
        .reply(cc.msg.id)
        .content(&help_msg)?
        .send()
        .await?;
    Ok(())
}
