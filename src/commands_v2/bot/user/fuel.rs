use twilight_util::builder::embed;

use crate::commands_v2::prelude::*;
use crate::utils::prelude::*;

/// Command: Calculate fuel required.
pub struct Fuel {
    args: Args,
}

impl Fuel {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands_v2::builder::*;

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
                    .max(10),
            )
            .option(
                number(
                    "seconds",
                    "Lap time seconds (and optionally milliseconds after a comma).",
                )
                .required()
                .min(0.0)
                .max(59.9999),
            )
            .option(
                number("consumption", "Fuel consumption in litres per lap.")
                    .required()
                    .min(0.1),
            )
            .dm()
    }

    async fn slash(ctx: Context, req: SlashRequest) -> CommandResult {
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

        ctx.http
            .create_message(req.interaction.channel_id.unwrap())
            .embeds(&[embed])?
            .send()
            .await?;

        Ok(Response::None)
    }
}
