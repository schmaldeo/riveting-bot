use crate::commands::{CommandContext, CommandError, CommandResult};
// use crate::utils::prelude::*;

/// Command: Voice channel controls.
pub async fn voice(cc: CommandContext<'_>) -> CommandResult {
    if cc.msg.guild_id.is_none() {
        return Err(CommandError::Disabled);
    }

    cc.http
        .create_message(cc.msg.channel_id)
        .reply(cc.msg.id)
        .content(&format!("```{}```", cc.cmd))?
        .send()
        .await?;

    Ok(())
}

/// Command: Tell the bot to connect to a voice channel.
pub async fn join(_cc: CommandContext<'_>) -> CommandResult {
    Ok(())
}

/// Command: Tell the bot to disconnect from a voice channel.
pub async fn leave(_cc: CommandContext<'_>) -> CommandResult {
    Ok(())
}
