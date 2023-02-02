use crate::commands_v2::prelude::*;
use crate::utils::prelude::*;

/// Command: Disconnect and shut down the bot.
pub struct Shutdown;

impl Shutdown {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands_v2::builder::*;

        command("shutdown", "Shutdown the bot.")
            .attach(Self::classic)
            .permissions(Permissions::MANAGE_GUILD)
            .dm()
    }

    pub async fn classic(ctx: Context, req: ClassicRequest) -> CommandResult {
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
