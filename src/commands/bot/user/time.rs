use chrono::{DateTime, Utc};
use htp::parser::ParseError;
use htp::HTPError;
use twilight_mention::timestamp::{Timestamp, TimestampStyle};
use twilight_mention::Mention;
use twilight_model::id::marker::ChannelMarker;
use twilight_model::id::Id;
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
pub struct Time {
    args: Args,
    channel_id: Option<Id<ChannelMarker>>,
}

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

    async fn uber(self, ctx: Context) -> CommandResult {
        let Some(channel_id) = self.channel_id else {
            return Err(CommandError::Disabled);
        };

        let expr = self
            .args
            .string("expression")
            .unwrap_or("now".to_string().into_boxed_str());

        let now = self
            .args
            .string("timezone")
            .and_then(|val| zone_to_now(&val).map_err(Into::into))
            .unwrap_or(Utc::now());

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
        let footer = format!("{} {}", long.mention(), relative.mention());

        let embed = embed::EmbedBuilder::new()
            .color(0xFFAA44)
            .field(EmbedFieldBuilder::new("Date & Time", long.mention().to_string()).inline())
            .field(EmbedFieldBuilder::new("Relative", relative.mention().to_string()).inline())
            .footer(EmbedFooterBuilder::new(footer))
            .build();

        ctx.http
            .create_message(channel_id)
            .embeds(&[embed])?
            .await?;

        Ok(Response::Clear)
    }

    async fn classic(ctx: Context, req: ClassicRequest) -> CommandResult {
        Self {
            args: req.args,
            channel_id: Some(req.message.channel_id),
        }
        .uber(ctx)
        .await
    }

    async fn slash(ctx: Context, req: SlashRequest) -> CommandResult {
        Self {
            args: req.args,
            channel_id: req.interaction.channel_id,
        }
        .uber(ctx)
        .await
    }
}

fn zone_to_now(zone: &str) -> AnyResult<DateTime<Utc>> {
    let tz = zone.trim().parse::<i64>()?;
    Utc::now()
        .checked_add_signed(chrono::Duration::hours(tz))
        .ok_or(anyhow::anyhow!("Failed to offset timezone"))
}
