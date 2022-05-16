use twilight_mention::parse::{MentionType, ParseMention};
use twilight_model::datetime::Timestamp;

use crate::commands::{CommandContext, CommandError, CommandResult};
use crate::parser;
use crate::utils::prelude::*;

/// Command: Silence voice users, or give a timeout.
pub async fn muter(cc: CommandContext<'_>) -> CommandResult {
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

/// Command: Silence a voice user for a set amount or random time.
pub async fn mute(cc: CommandContext<'_>) -> CommandResult {
    let Some(guild_id) = cc.msg.guild_id else {
        return Err(CommandError::Disabled)
    };

    let timeout = 30;

    let (target, rest) = parser::maybe_quoted_arg(cc.args.trim())?;
    parser::ensure_rest_is_empty(rest)?;

    let mention = MentionType::parse(target)
        .map_err(|_| CommandError::UnexpectedArgs(format!("Failed to parse user: '{}'", target)))?;

    let target_user_id = match mention {
        MentionType::User(user_id) => Ok(user_id),
        MentionType::Channel(_) => Err(CommandError::UnexpectedArgs(
            "Expected user tag, got channel".to_string(),
        )),
        MentionType::Emoji(_) => Err(CommandError::UnexpectedArgs(
            "Expected user tag, got emoji".to_string(),
        )),
        MentionType::Role(_) => Err(CommandError::UnexpectedArgs(
            "Expected user tag, got role".to_string(),
        )),
        MentionType::Timestamp(_) => Err(CommandError::UnexpectedArgs(
            "Expected user tag, got timestamp".to_string(),
        )),
        e => Err(CommandError::UnexpectedArgs(format!(
            "Expected user tag, got '{e}'"
        ))),
    }?;

    cc.http
        .update_guild_member(guild_id, target_user_id)
        .mute(true)
        .exec()
        .await?;

    tokio::time::sleep(std::time::Duration::from_secs(timeout)).await;

    cc.http
        .update_guild_member(guild_id, target_user_id)
        .mute(false)
        .exec()
        .await?;

    Ok(())
}

/// Command: Give someone a timeout (target cannot be an admin or guild owner).
pub async fn timeout(cc: CommandContext<'_>) -> CommandResult {
    let Some(guild_id) = cc.msg.guild_id else {
        return Err(CommandError::Disabled)
    };

    let timeout = 30;

    let now = chrono::Utc::now().timestamp();
    let until = Timestamp::from_secs(now + timeout).unwrap();

    println!("now: {:?}, until: {:?}", now, until.as_secs());

    let mention = MentionType::parse(cc.args.trim()).unwrap();
    let target_user_id = match mention {
        MentionType::User(user_id) => Ok(user_id),
        MentionType::Channel(_) => Err(CommandError::UnexpectedArgs(
            "Expected user tag, got channel".to_string(),
        )),
        MentionType::Emoji(_) => Err(CommandError::UnexpectedArgs(
            "Expected user tag, got emoji".to_string(),
        )),
        MentionType::Role(_) => Err(CommandError::UnexpectedArgs(
            "Expected user tag, got role".to_string(),
        )),
        MentionType::Timestamp(_) => Err(CommandError::UnexpectedArgs(
            "Expected user tag, got timestamp".to_string(),
        )),
        e => Err(CommandError::UnexpectedArgs(format!(
            "Expected user tag, got '{e}'"
        ))),
    }?;

    cc.http
        .update_guild_member(guild_id, target_user_id)
        .communication_disabled_until(Some(until)) // This gives permissions error?
        .unwrap()
        .exec()
        .await?;

    Ok(())
}
