use twilight_util::builder::embed;

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
                integer("stint", "Length of the race or stint in minutes.")
                    .required()
                    .min(1),
            )
            .option(
                integer("minutes", "Lap time minutes.")
                    .required()
                    .min(0)
                    .max(30),
            )
            .option(
                number(
                    "seconds",
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
        let stint = req.args.integer("stint")?;
        let minutes = req.args.integer("minutes")?;
        let seconds = req.args.number("seconds")?;
        let consumption = req.args.number("consumption")?;

        let length_in_seconds = (stint * 60) as f64;
        let laptime_in_seconds = (minutes * 60) as f64 + seconds;

        let amount_of_laps = (length_in_seconds / laptime_in_seconds).ceil();
        let fuel_needed = amount_of_laps * consumption;

        let embed = embed::EmbedBuilder::new()
            .title("Fuel needed")
            .field(
                embed::EmbedFieldBuilder::new("Min litres: ", fuel_needed.ceil().to_string())
                    .inline(),
            )
            .field(
                embed::EmbedFieldBuilder::new(
                    "Recommended litres: ",
                    (fuel_needed + consumption).ceil().to_string(),
                )
                .inline(),
            )
            .field(embed::EmbedFieldBuilder::new("Laps: ", amount_of_laps.to_string()).inline())
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
