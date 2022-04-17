use std::str::SplitWhitespace;

use twilight_model::channel::Message;

use crate::commands::CommandFunction;
use crate::utils::*;
use crate::Context;

/// Command: Ping Pong!
#[derive(Debug, Default)]
pub struct Ping;

#[async_trait]
impl CommandFunction for Ping {
    async fn execute(
        &self,
        ctx: &Context,
        msg: &Message,
        _args: SplitWhitespace<'_>,
    ) -> AnyResult<()> {
        ctx.http
            .create_message(msg.channel_id)
            .content("Pong!")?
            .exec()
            .await?;
        Ok(())
    }
}
