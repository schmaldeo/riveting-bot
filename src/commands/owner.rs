use std::str::SplitWhitespace;

use twilight_model::channel::Message;

use crate::commands::CommandFunction;
use crate::utils::*;
use crate::Context;

/// Command: Disconnect and shut down the bot.
#[derive(Debug, Default)]
pub struct Shutdown;

#[async_trait]
impl CommandFunction for Shutdown {
    async fn execute(
        &self,
        ctx: &Context,
        msg: &Message,
        _args: SplitWhitespace<'_>,
    ) -> AnyResult<()> {
        info!("Shutting down by chat command");

        ctx.http
            .create_message(msg.channel_id)
            .content("Shutting down...")?
            .send()
            .await?;

        // Shut down the cluster, sessions will not be resumable.
        ctx.cluster.down();

        Ok(())
    }
}
