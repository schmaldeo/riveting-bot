use twilight_model::datetime::Timestamp;
use twilight_model::id::Id;

use crate::commands::{CommandContext, CommandError, CommandResult};
use crate::utils::*;

/// Command: Silence voice users.
pub async fn muter(cc: CommandContext<'_>) -> CommandResult {
    Ok(())
}

/// Command: Disable communications and mute a person for a minute.
pub async fn timeout(cc: CommandContext<'_>) -> CommandResult {
    let Some(guild_id) = cc.msg.guild_id else {
        return Err(CommandError::Disabled)
    };

    let timeout = 60;
    let now = chrono::Utc::now().timestamp();
    let until = Timestamp::from_secs(now + timeout).unwrap();

    println!("now: {:?}, until: {:?}", now, until.as_secs());

    let target_user_id = Id::new(0);

    cc.http
        .update_guild_member(guild_id, target_user_id)
        .mute(true)
        .communication_disabled_until(Some(until))
        .unwrap()
        .exec()
        .await?;

    // TEMP Delete me.
    let member = cc
        .http
        .guild_member(guild_id, target_user_id)
        .send()
        .await?;

    println!("dis: {:?}", member.communication_disabled_until);

    Ok(())
}
