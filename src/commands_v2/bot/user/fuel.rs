use twilight_util::builder::embed;

use crate::commands_v2::prelude::*;
use crate::utils::prelude::*;

/// Command: Calculate fuel required.
#[derive(Default)]
pub struct Fuel {
    args: Args,
}

impl Fuel {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(ctx: Context, req: SlashRequest) -> CommandResult {
        let length = req.args.get("length").integer().unwrap();
        let minutes = req.args.get("minutes").integer().unwrap();
        let seconds = req.args.get("seconds").number().unwrap();
        let fuel_consumption = req.args.get("fuel_consumption").number().unwrap();

        let length_in_seconds = (length * 60) as f64;
        let laptime_in_seconds = (minutes * 60) as f64 + seconds;

        let amount_of_laps = (length_in_seconds / laptime_in_seconds).ceil();
        let fuel_needed = amount_of_laps * fuel_consumption;

        let embed = embed::EmbedBuilder::new()
            .title("Fuel needed")
            .field(
                embed::EmbedFieldBuilder::new("Min litres: ", fuel_needed.ceil().to_string())
                    .inline(),
            )
            .field(
                embed::EmbedFieldBuilder::new(
                    "Recommended litres: ",
                    (fuel_needed + fuel_consumption).ceil().to_string(),
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
