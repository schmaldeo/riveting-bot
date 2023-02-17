use crate::commands_v2::prelude::*;
use crate::utils::prelude::*;

/// Command: Disconnect and shut down the bot.
pub struct Shutdown;

impl Shutdown {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands_v2::builder::*;

        command("shutdown", "Shutdown the bot.")
            .attach(Self::classic)
            .dm()
    }

    async fn classic(ctx: Context, req: ClassicRequest) -> CommandResult {
        // Owner check (not done by command handling).
        let sender_id = req.message.author.id;
        let ok = if let Some(owner) = &ctx.application.owner {
            owner.id == sender_id
        } else if let Some(team) = &ctx.application.team {
            team.members.iter().any(|m| m.user.id == sender_id)
        } else {
            false
        };

        if !ok {
            return Ok(Response::None);
        }

        info!("Shutting down by chat command");

        ctx.http
            .create_message(req.message.channel_id)
            .content("Shutting down...")?
            .send()
            .await?;

        // Shut down the cluster, sessions will not be resumable.
        ctx.cluster.down();

        Ok(Response::None)
    }
}
