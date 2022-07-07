use htp::parser::ParseError;
use htp::HTPError;
use twilight_mention::timestamp::{Timestamp, TimestampStyle};
use twilight_mention::Mention;

use crate::commands::{CommandContext, CommandError, CommandResult};
use crate::utils::prelude::*;

/// Command: Display a discord timestamp.
pub async fn time(cc: CommandContext<'_>) -> CommandResult {
    if cc.args.trim().is_empty() {
        // Show command help.
        cc.http
            .create_message(cc.msg.channel_id)
            .reply(cc.msg.id)
            .content(&format!("```{}```", cc.cmd))?
            .send()
            .await?;

        return Ok(());
    }

    let now = chrono::Utc::now();
    let parsed = htp::parse_time_clue(cc.args.trim(), now, true).map_err(|e| {
        if let HTPError::ParseError(ParseError::PestError(_)) = e {
            CommandError::ParseError("Failed to parse time".to_string())
        } else {
            CommandError::ParseError(format!("Failed to parse time: {e}"))
        }
    })?;

    let unix = parsed.timestamp() as _;
    let long = Timestamp::new(unix, Some(TimestampStyle::LongDateTime));
    let relative = Timestamp::new(unix, Some(TimestampStyle::RelativeTime));

    cc.http
        .create_message(cc.msg.channel_id)
        .reply(cc.msg.id)
        .content(&format!("{}\n{}", long.mention(), relative.mention()))?
        .send()
        .await?;

    Ok(())
}
