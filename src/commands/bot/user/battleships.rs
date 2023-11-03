use pyo3::prelude::*;
use crate::utils::prelude::*;
use twilight_model::http::permission_overwrite::{PermissionOverwrite, PermissionOverwriteType};
use twilight_model::guild::Permissions;
use twilight_model::channel::Channel;
use twilight_util::builder::embed::{self, EmbedFieldBuilder, ImageSource};

use crate::commands::prelude::*;

pub struct Battleships;

impl Battleships {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands::builder::*;

        command("battleships", "Playe a game of battleships.")
            .attach(Self::slash)
            .option(user("user", "User to play against").required())
            .dm()
    }

    async fn slash(ctx: Context, req: SlashRequest) -> CommandResponse {
        let player1 = req.interaction.author_id().ok_or(CommandError::MissingArgs)?;
        let player2 = req.args.user("user")?.unwrap_id();
        let players = [player1, player2];
        let guild_id = req.interaction.guild_id.unwrap();

        let everyone_permission_overwrite = PermissionOverwrite { allow: (None), deny: (Some(Permissions::VIEW_CHANNEL)), id: (guild_id.cast()), kind: (PermissionOverwriteType::Role) };

        let mut channels: Vec<Channel> = Vec::new();
        for (index, player) in players.into_iter().enumerate() {
          let channel = ctx.http.create_guild_channel(guild_id, &format!("Player {}", index + 1)).unwrap().send().await?;
          let player_permission_overwrite = PermissionOverwrite { allow: (Some(Permissions::VIEW_CHANNEL)), deny: (None), id: (player.cast()), kind: (PermissionOverwriteType::Member) };
          ctx.http.update_channel_permission(channel.id, &everyone_permission_overwrite).await?;
          ctx.http.update_channel_permission(channel.id, &player_permission_overwrite).await?;
          channels.push(channel);
        }

        let code = include_str!("engine.py");
        let strong = Python::with_gil(|py| -> PyResult<String> {
            let module = PyModule::from_code(py, code, "engine.py", "engine")?;
            let player = module.getattr("Player")?.call1((7, 7))?;
            let ship_types = module.getattr("ShipType")?.getattr("DESTROYER")?;
            player.call_method1("random_spawn", (ship_types,))?;
            let board = player.call_method0("get_stringified_board")?;
            let res = board.extract::<String>()?;

            Ok(res)
        }).unwrap();

        let embed = embed::EmbedBuilder::new()
            .title("Battleships")
            .color(0x9500a8)
            .field(EmbedFieldBuilder::new("Board", format!("```{}```", strong)))
            .build();

        ctx.http.create_message(channels[0].id).embeds(&[embed]).unwrap().await?;

        Ok(Response::clear(ctx, req))
    }
}
