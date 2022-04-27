use crate::commands::{CommandContext, CommandError, CommandResult};
use crate::utils::*;

/// Command: Voice channel controls.
pub async fn voice(cc: CommandContext<'_>) -> CommandResult {
    // TODO Display help.
    if !cc.args.is_empty() {
        return Err(CommandError::NotImplemented);
    }
    Ok(())
}

/// Command: Tell the bot to connect to a voice channel.
pub async fn join(cc: CommandContext<'_>) -> CommandResult {
    // songbird::Call::
    Ok(())
}

/// Command: Tell the bot to disconnect from a voice channel.
pub async fn leave(cc: CommandContext<'_>) -> CommandResult {
    Ok(())
}
