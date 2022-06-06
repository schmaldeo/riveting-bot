use twilight_util::builder::embed;

use crate::commands::{CommandContext, CommandError, CommandResult};
use crate::parser;
use crate::utils::prelude::*;

/// Command: Calculate fuel required.
/// Usage: fuel <length: minutes> <laptime> <fuel per lap>
pub async fn fuel(cc: CommandContext<'_>) -> CommandResult {
    let args = parser::parse_args(cc.args.trim())?;

    let length = match args.get(0) {
        Some(l) => l,
        None => {
            cc.http
                .create_message(cc.msg.channel_id)
                .reply(cc.msg.id)
                .content(&format!("```{}```", cc.cmd))?
                .send()
                .await?;

            return Err(CommandError::MissingArgs);
        },
    }
    .trim();

    let laptime = args.get(1).ok_or(CommandError::MissingArgs)?.trim();
    let fuel_per_lap = args.get(2).ok_or(CommandError::MissingArgs)?.trim();
    parser::ensure_rest_is_empty(args.get(3).copied())?;

    let length = length.parse::<u16>().unwrap();
    let laptime = laptime
        .split(&[':', '.'])
        .map(|lol| lol.parse::<u16>().unwrap())
        .collect::<Vec<_>>();
    let fuel_per_lap = fuel_per_lap.parse::<f32>().unwrap();

    let length_in_sec = (length * 60) as f32;
    let laptime_in_sec = (laptime[0] * 60 + laptime[1]) as f32 + (laptime[2] as f32) / 1000.0;
    let amount_of_laps = (length_in_sec / laptime_in_sec).ceil();
    let fuel_needed = amount_of_laps * fuel_per_lap;

    let embed = embed::EmbedBuilder::new()
        .title("Fuel needed")
        .field(
            embed::EmbedFieldBuilder::new("Min litres: ", fuel_needed.ceil().to_string()).inline(),
        )
        .field(
            embed::EmbedFieldBuilder::new(
                "Recommended litres: ",
                (fuel_needed + fuel_per_lap).ceil().to_string(),
            )
            .inline(),
        )
        .field(embed::EmbedFieldBuilder::new("Laps: ", amount_of_laps.to_string()).inline())
        .color(0x39fc32)
        .build();

    cc.http
        .create_message(cc.msg.channel_id)
        .reply(cc.msg.id)
        .embeds(&[embed])?
        .send()
        .await?;
    Ok(())
}
