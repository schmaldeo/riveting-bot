use rand::random;

use crate::commands::prelude::*;

/// Command: Coinflip.
pub struct Coinflip;

impl Coinflip {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands::builder::*;

        command("coinflip", "Flip a coin.").attach(Self::slash)
    }

    async fn slash(ctx: Context, req: SlashRequest) -> CommandResponse {
        let flip = random::<bool>();
        let flip = if flip { "> Heads" } else { "> Tails" };

        ctx.interaction()
            .create_followup(&req.interaction.token)
            .content(flip)?
            .await?;

        Ok(Response::none())
    }
}
