use crate::commands::{CommandContext, CommandError, CommandResult};
use crate::utils::prelude::*;

pub mod alias;
pub mod config;
pub mod muter;
pub mod roles;

/// Command: Delete a bunch of messages at once.
#[cfg(feature = "bulk-delete")]
pub async fn delete_messages(cc: CommandContext<'_>) -> CommandResult {
    const TWO_WEEKS_SECS: i64 = 60 * 60 * 24 * 7 * 2;

    let two_weeks_ago = cc.msg.timestamp.as_secs() - TWO_WEEKS_SECS;

    let mut delete_count = 100;

    if !cc.args.is_empty() {
        delete_count = cc.args.parse().map_err(|_| {
            CommandError::UnexpectedArgs("Could not parse delete count".to_string())
        })?;
    }

    // Fetch and filter messages that are not older than two weeks.
    let msgs: Vec<_> = cc
        .http
        .channel_messages(cc.msg.channel_id)
        .around(cc.msg.id)
        .limit(delete_count)?
        .send()
        .await?
        .into_iter()
        .filter(|m| two_weeks_ago < m.timestamp.as_secs())
        .map(|m| m.id)
        .collect();

    // Delete the messages.
    if msgs.len() > 1 {
        // Bulk delete must have 2 to 100 messages.
        cc.http
            .delete_messages(cc.msg.channel_id, &msgs)
            .exec()
            .await?;
    } else {
        // Delete only the sent message itself.
        cc.http
            .delete_message(cc.msg.channel_id, cc.msg.id)
            .exec()
            .await?;
    }

    Ok(())
}
