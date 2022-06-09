use crate::commands::{CommandContext, CommandError, CommandResult};
use crate::utils::prelude::*;

/// Command: Create or edit bot messages.
pub async fn bot(cc: CommandContext<'_>) -> CommandResult {
    if cc.msg.guild_id.is_none() {
        return Err(CommandError::Disabled);
    }

    cc.http
        .create_message(cc.msg.channel_id)
        .reply(cc.msg.id)
        .content(&format!("```{}```", cc.cmd))?
        .exec()
        .await?;

    Ok(())
}

/// Command: Post a message as the bot.
pub async fn say(cc: CommandContext<'_>) -> CommandResult {
    if cc.msg.guild_id.is_none() {
        return Err(CommandError::Disabled);
    }

    let empty = cc.args.trim().is_empty();
    let content = if empty { "no u" } else { cc.args };

    let msg = cc
        .http
        .create_message(cc.msg.channel_id)
        .content(content)?
        .send()
        .await?;

    info!("Bot message created with id '{}'", msg.id);

    Ok(())
}

/// Command: Edit a message created by the bot (can be anything).
pub async fn edit(cc: CommandContext<'_>) -> CommandResult {
    if cc.msg.guild_id.is_none() {
        return Err(CommandError::Disabled);
    }

    let Some(replied) = &cc.msg.referenced_message else {
        return Err(CommandError::MissingReply);
    };

    // Ignore if replied message is not from this bot.
    if replied.author.id != cc.user.id {
        return Err(CommandError::UnexpectedArgs(
            "Replied message is not from this bot".to_string(),
        ));
    }

    if cc.args.trim().is_empty() {
        return Err(CommandError::MissingArgs);
    }

    cc.http
        .update_message(replied.channel_id, replied.id)
        .content(Some(cc.args))?
        .exec()
        .await?;

    info!("Bot message edited with id '{}'", replied.id);

    Ok(())
}
