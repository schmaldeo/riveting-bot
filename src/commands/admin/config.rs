use crate::commands::{CommandContext, CommandError, CommandResult};
use crate::utils::*;

/// Command: Manage guild configuration.
pub async fn config(cc: CommandContext<'_>) -> CommandResult {
    Err(CommandError::NotImplemented)
}

/// Command: Get a guild configuration value.
pub async fn get(cc: CommandContext<'_>) -> CommandResult {
    Err(CommandError::NotImplemented)
}

/// Command: Set a guild configuration value.
pub async fn set(cc: CommandContext<'_>) -> CommandResult {
    let Some((setting, value)) = cc.args.split_once(|c: char| c.is_whitespace()) else {
        return Err(CommandError::MissingArgs)
    };

    match setting {
        "prefix" => {
            // TODO Strip any quotes and add a confirmation.

            let guild_id = cc.msg.guild_id.unwrap();
            let mut lock = cc.config.lock().unwrap();

            lock.set_prefix(guild_id, value.trim()); // Set prefix for this guild.
            lock.write()?; // Update config file.
        },
        _ => return Err(CommandError::NotImplemented),
    }

    Ok(())
}
