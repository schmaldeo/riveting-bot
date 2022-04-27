use crate::commands::{CommandContext, CommandResult};
use crate::utils::*;

/// Command: Disconnect and shut down the bot.
pub async fn shutdown(cc: CommandContext<'_>) -> CommandResult {
    info!("Shutting down by chat command");

    cc.http
        .create_message(cc.msg.channel_id)
        .content("Shutting down...")?
        .send()
        .await?;

    // Shut down the cluster, sessions will not be resumable.
    cc.cluster.down();

    Ok(())
}
