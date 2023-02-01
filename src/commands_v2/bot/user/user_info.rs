use twilight_util::builder::embed::{self, ImageSource};

use crate::commands_v2::arg::Ref;
use crate::commands_v2::prelude::*;
use crate::utils::prelude::*;

/// Command: Get information about user.
#[derive(Default)]
pub struct UserInfo {
    args: Args,
}

impl UserInfo {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(ctx: Context, req: SlashRequest) -> CommandResult {
        // If no args provided, check own props
        let user_id = match req.args.get("user").user() {
            Some(Ref::Id(user_to_get)) => user_to_get,
            _ => req.interaction.author_id().unwrap(),
        };

        let request = ctx.http.user(user_id).send().await.unwrap();
        let username = request.name;

        let embed = embed::EmbedBuilder::new()
            .title(username)
            .color(request.accent_color.unwrap_or(0));

        // TODO get role names from these markers
        // let roles = ctx
        //     .http
        //     .guild_member(req.interaction.guild_id.unwrap(), user_id)
        //     .send()
        //     .await
        //     .unwrap()
        //     .roles;

        // println!("{:?}", roles);

        // If no avatar for the user, get the default one
        let url = match request.avatar {
            Some(avatar) => {
                format!("https://cdn.discordapp.com/avatars/{user_id}/{avatar}.png?size=2048")
            },
            _ => "https://cdn.discordapp.com/embed/avatars/0.png".to_string(),
        };

        let image_source = ImageSource::url(url).unwrap();
        let embed = embed.image(image_source).build();

        ctx.http
            .create_message(req.interaction.channel_id.unwrap())
            .embeds(&[embed])?
            .send()
            .await?;

        Ok(Response::Clear)
    }
}
