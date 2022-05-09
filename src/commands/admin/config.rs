use indoc::formatdoc;

use crate::commands::{CommandContext, CommandError, CommandResult};
use crate::parser;
use crate::utils::prelude::*;

/// Command: Manage guild configuration.
pub async fn config(cc: CommandContext<'_>) -> CommandResult {
    if !cc.args.is_empty() {
        return Err(CommandError::NotImplemented);
    }

    // For now this will force reload the config from file.
    let mut lock = cc.config.lock().unwrap();
    lock.reload()?;

    Ok(())
}

/// Command: Get a guild configuration value.
pub async fn get(cc: CommandContext<'_>) -> CommandResult {
    let Some(guild_id) = cc.msg.guild_id else {
        return Err(CommandError::Disabled)
    };

    parser::ensure_rest_is_empty(Some(cc.args.trim()))?;

    let response = {
        let lock = cc.config.lock().unwrap();
        let guild = &lock.guild(guild_id).ok_or_else(|| {
            CommandError::UnknownResource("No custom configuration found".to_string())
        })?;

        formatdoc!(
            "
            Guild uses prefix: `{}`
            Guild's aliases: 
            ```yaml
            {:#?}
            ```",
            guild.prefix(),
            guild.aliases()
        )
    };

    cc.http
        .create_message(cc.msg.channel_id)
        .reply(cc.msg.id)
        .content(&response)?
        .send()
        .await?;

    Ok(())
}

/// Command: Set a guild configuration value.
pub async fn set(cc: CommandContext<'_>) -> CommandResult {
    let Some(guild_id) = cc.msg.guild_id else {
        return Err(CommandError::Disabled)
    };

    let args = parser::parse_args(cc.args.trim())?;

    let setting = args.get(0).ok_or(CommandError::MissingArgs)?.trim();
    let value = args.get(1).ok_or(CommandError::MissingArgs)?.trim();

    parser::ensure_rest_is_empty(args.get(3).copied())?;

    match setting {
        "prefix" => {
            // TODO Add a confirmation.

            let mut lock = cc.config.lock().unwrap();

            lock.set_prefix(guild_id, value); // Set prefix for this guild.
            lock.write()?; // Update config file.
        },
        _ => return Err(CommandError::NotImplemented),
    }

    Ok(())
}
