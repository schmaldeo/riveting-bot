use crate::commands::{CommandContext, CommandError, CommandResult};
use crate::utils::*;

/// Command: Manage guild command aliases.
pub async fn alias(cc: CommandContext<'_>) -> CommandResult {
    Err(CommandError::NotImplemented)
}

/// Command: Get a guild command alias definition.
pub async fn get(cc: CommandContext<'_>) -> CommandResult {
    Err(CommandError::NotImplemented)
}

/// Command: Set a guild command alias definition.
pub async fn set(cc: CommandContext<'_>) -> CommandResult {
    let Some(guild_id) = cc.msg.guild_id else {
        return Err(CommandError::NotImplemented)
    };

    Ok(())
}
