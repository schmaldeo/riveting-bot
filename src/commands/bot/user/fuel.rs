use chrono::NaiveTime;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder, EmbedFooterBuilder};

use crate::commands::prelude::*;
use crate::utils::prelude::*;

/// Command: Calculate fuel required.
pub struct Fuel;

impl Fuel {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands::builder::*;

        command("fuel", "Calculate race fuel required.")
            .attach(Self::slash)
            .option(
                integer("stint-minutes", "Length of the race or stint in minutes.")
                    .required()
                    .min(1),
            )
            .option(
                integer("lap-minutes", "Lap time minutes.")
                    .required()
                    .min(0)
                    .max(30),
            )
            .option(
                number(
                    "lap-seconds",
                    "Lap time seconds (and optionally milliseconds as decimal).",
                )
                .required()
                .min(0.0)
                .max(59.9999),
            )
            .option(
                number("consumption", "Fuel consumption in litres per lap.")
                    .required()
                    .min(0.1)
                    .max(100.0),
            )
            .dm()
    }

    async fn slash(ctx: Context, req: SlashRequest) -> CommandResponse {
        let stint = req.args.integer("stint-minutes")?;
        let minutes = req.args.integer("lap-minutes")?;
        let seconds = req.args.number("lap-seconds")?;
        let consumption = req.args.number("consumption")?;

        let length_in_seconds = (stint * 60) as f64;
        let laptime_in_seconds = (minutes * 60) as f64 + seconds;

        let amount_of_laps = (length_in_seconds / laptime_in_seconds).ceil();
        let fuel_needed = amount_of_laps * consumption;

        let embed = EmbedBuilder::new()
            .title(":fuelpump: Fuel kalkulus")
            .field(EmbedFieldBuilder::new("Minimum", fuel_needed.ceil().to_string()).inline())
            .field(
                EmbedFieldBuilder::new(
                    "Recommended",
                    (fuel_needed + consumption).ceil().to_string(),
                )
                .inline(),
            )
            .field(EmbedFieldBuilder::new("Laps", amount_of_laps.to_string()).inline())
            .footer(EmbedFooterBuilder::new(format!(
                "Stint: {stint}, Laptime: {laptime}, Usage: {consumption}",
                stint = NaiveTime::from_hms_opt((stint / 60) as u32, (stint % 60) as u32, 0)
                    .unwrap_or_default(),
                laptime = NaiveTime::from_hms_milli_opt(
                    (minutes / 60) as u32,
                    (minutes % 60) as u32,
                    seconds.trunc() as u32,
                    (seconds.fract() * 1000.0) as u32
                )
                .unwrap_or_default()
                .format("%M:%S%.3f"),
            )))
            .color(0xDB3DBE)
            .build();

        ctx.interaction()
            .update_response(&req.interaction.token)
            .embeds(Some(&[embed]))?
            .send()
            .await?;

        Ok(Response::none())
    }
}
