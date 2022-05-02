use crate::commands::{CommandContext, CommandError, CommandResult};
// use crate::utils::prelude::*;

/// Command: Voice channel controls.
pub async fn voice(cc: CommandContext<'_>) -> CommandResult {
    if !cc.args.is_empty() {
        // TODO Display help.
        return Err(CommandError::NotImplemented);
    }

    Err(CommandError::NotImplemented)
}

/// Command: Tell the bot to connect to a voice channel.
pub async fn join(_cc: CommandContext<'_>) -> CommandResult {
    Ok(())
}

/// Command: Tell the bot to disconnect from a voice channel.
pub async fn leave(_cc: CommandContext<'_>) -> CommandResult {
    Ok(())
}
