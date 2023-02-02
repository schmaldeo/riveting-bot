use htp::parser::ParseError;
use htp::HTPError;
use twilight_mention::timestamp::{Timestamp, TimestampStyle};
use twilight_mention::Mention;
use twilight_model::id::marker::{ChannelMarker, MessageMarker};
use twilight_model::id::Id;

use crate::commands_v2::prelude::*;
use crate::utils::prelude::*;

/*
// TODO Try `event_parser`

HTP examples:
    * 4 min ago, 4 h ago, 1 week ago, in 2 hours, in 1 month
    * last friday at 19, monday at 6 am
    * 7, 7am, 7pm, 7:30, 19:43:00
    * now, yesterday, today, friday
    * 2020-12-25T19:43:00
*/

/// Command: Display a discord timestamp.
pub struct Time {
    args: Args,
    channel_id: Option<Id<ChannelMarker>>,
    message_id: Option<Id<MessageMarker>>,
}

impl Time {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands_v2::builder::*;

        command("time", "Display a discord timestamp.")
            .attach(Self::classic)
            .attach(Self::slash)
            .option(string("expression", "Time expression to evaluate."))
            .dm()
    }

    pub async fn uber(self, ctx: Context) -> CommandResult {
        let Some(expr) = self.args.get("expression").string() else {
            return Err(CommandError::MissingArgs);
        };

        let Some(channel_id) = self.channel_id else {
            return Err(CommandError::MissingArgs);
        };

        let Some(message_id) = self.message_id else {
            return Err(CommandError::MissingArgs); // FIXME: Slash command has no message id.
        };

        let now = chrono::Utc::now();
        let parsed = htp::parse_time_clue(expr.trim(), now, true).map_err(|e| {
            if let HTPError::ParseError(ParseError::PestError(_)) = e {
                CommandError::ParseError("Failed to parse time".to_string())
            } else {
                CommandError::ParseError(format!("Failed to parse time: {e}"))
            }
        })?;

        let unix = parsed.timestamp() as _;
        let long = Timestamp::new(unix, Some(TimestampStyle::LongDateTime));
        let relative = Timestamp::new(unix, Some(TimestampStyle::RelativeTime));

        ctx.http
            .create_message(channel_id)
            .reply(message_id)
            .content(&format!("{}\n{}", long.mention(), relative.mention()))?
            .send()
            .await?;

        Ok(Response::Clear)
    }

    pub async fn classic(ctx: Context, req: ClassicRequest) -> CommandResult {
        Self {
            args: req.args,
            channel_id: Some(req.message.channel_id),
            message_id: Some(req.message.id),
        }
        .uber(ctx)
        .await
    }

    pub async fn slash(ctx: Context, req: SlashRequest) -> CommandResult {
        Self {
            args: req.args,
            channel_id: req.interaction.channel_id,
            message_id: None,
        }
        .uber(ctx)
        .await
    }
}
