use chrono::{DateTime, FixedOffset, Utc};
use htp::parser::ParseError;
use htp::HTPError;
use twilight_mention::timestamp::{Timestamp, TimestampStyle};
use twilight_mention::Mention;
use twilight_model::channel::message::Embed;
use twilight_util::builder::embed::{self, EmbedFieldBuilder, EmbedFooterBuilder};

use crate::commands::prelude::*;
use crate::utils::prelude::*;

/*
// TODO Try `event_parser / date_time_parser`.
// TODO Show these examples somewhere as help text.
HTP examples:
    * 4 min ago, 4 h ago, 1 week ago, in 2 hours, in 1 month
    * last friday at 19, monday at 6 am
    * 7, 7am, 7pm, 7:30, 19:43:00
    * now, yesterday, today, friday
    * 2020-12-25T19:43:00
*/

/// If your timezone is something else, unlucky.
const TIMEZONES: [(&str, &str); 24] = [
    ("UTC-11", "-11"),
    ("UTC-10", "-10"),
    ("UTC-9", "-9"),
    ("UTC-8", "-8"),
    ("UTC-7", "-7"),
    ("UTC-6", "-6"),
    ("UTC-5", "-5"),
    ("UTC-4", "-4"),
    ("UTC-3", "-3"),
    ("UTC-2", "-2"),
    ("UTC-1", "-1"),
    ("UTC+0", "+0"),
    ("UTC+1", "+1"),
    ("UTC+2", "+2"),
    ("UTC+3", "+3"),
    ("UTC+4", "+4"),
    ("UTC+5", "+5"),
    ("UTC+6", "+6"),
    ("UTC+7", "+7"),
    ("UTC+8", "+8"),
    ("UTC+9", "+9"),
    ("UTC+10", "+10"),
    ("UTC+11", "+11"),
    ("UTC+12", "+12"),
];

/// Command: Display a discord timestamp.
pub struct Time;

impl Time {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands::builder::*;

        command("time", "Display a discord timestamp.")
            .attach(Self::classic)
            .attach(Self::slash)
            .option(string("expression", "Time expression to evaluate."))
            .option(string("timezone", "Your timezone offset.").choices(TIMEZONES))
            .dm()
    }

    async fn uber(args: Args) -> CommandResult<Embed> {
        let expr = args
            .string("expression")
            .unwrap_or_else(|_| "now".to_string().into_boxed_str());

        let now = args
            .string("timezone")
            .and_then(|val| Ok(timezone(&val)?))
            .unwrap_or_else(|_| Utc::now().into());

        let parsed = htp::parse_time_clue(expr.trim(), now, true).map_err(|e| {
            if let HTPError::ParseError(ParseError::PestError(_)) = e {
                CommandError::ParseError("Failed to parse datetime".to_string())
            } else {
                CommandError::ParseError(format!("Failed to parse datetime: {e}"))
            }
        })?;

        let unix = parsed.timestamp() as _;
        let long = Timestamp::new(unix, Some(TimestampStyle::LongDateTime));
        let relative = Timestamp::new(unix, Some(TimestampStyle::RelativeTime));
        let footer = format!("Copypasta: {} {}", long.mention(), relative.mention());

        Ok(embed::EmbedBuilder::new()
            .color(0xFFAA44)
            .field(EmbedFieldBuilder::new("Date & Time", long.mention().to_string()).inline())
            .field(EmbedFieldBuilder::new("Relative", relative.mention().to_string()).inline())
            .footer(EmbedFooterBuilder::new(footer))
            .build())
    }

    async fn classic(ctx: Context, req: ClassicRequest) -> CommandResponse {
        let embed = Self::uber(req.args).await?;

        ctx.http
            .create_message(req.message.channel_id)
            .reply(req.message.id)
            .embeds(&[embed])?
            .await?;

        Ok(Response::none())
    }

    async fn slash(ctx: Context, req: SlashRequest) -> CommandResponse {
        let embed = Self::uber(req.args).await?;

        ctx.interaction()
            .update_response(&req.interaction.token)
            .embeds(Some(&[embed]))?
            .await?;

        Ok(Response::none())
    }
}

fn timezone(zone: &str) -> AnyResult<DateTime<FixedOffset>> {
    let hour = 3600; // Seconds.
    let offset = hour * zone.trim().parse::<i32>()?;
    let offset = FixedOffset::east_opt(offset)
        .or_else(|| FixedOffset::west_opt(offset))
        .ok_or_else(|| anyhow::anyhow!("Invalid timezone offset"))?;
    Ok(Utc::now().with_timezone(&offset))
}
