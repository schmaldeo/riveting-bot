use std::mem;

use indoc::formatdoc;
use twilight_model::id::marker::{ChannelMarker, MessageMarker};
use twilight_model::id::Id;

use crate::commands::{CommandContext, CommandError, CommandResult};
use crate::parser;
use crate::utils::prelude::*;

/// Command: Manage guild configuration.
pub async fn config(cc: CommandContext<'_>) -> CommandResult {
    if !cc.args.is_empty() {
        return Err(CommandError::NotImplemented);
    }

    {
        // For now this will force reload the config from file.
        let mut lock = cc.config.lock().unwrap();
        lock.reload()?;
    }

    cc.http
        .create_message(cc.msg.channel_id)
        .reply(cc.msg.id)
        .content(&format!("```{}```", cc.cmd))?
        .send()
        .await?;

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

    let setting = args.first().ok_or(CommandError::MissingArgs)?.trim();
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

/// Command: Clean config from dangling id references and other expired things.
/// # Notes
/// This will also clean up aliases that could be valid with certain features enabled, but at the runtime are not.
pub async fn cleanup(cc: CommandContext<'_>) -> CommandResult {
    let Some(guild_id) = cc.msg.guild_id else {
        return Err(CommandError::NotImplemented)
    };

    parser::ensure_rest_is_empty(Some(cc.args.trim()))?;

    info!("Cleaning up config for guild: '{guild_id}'");

    let mut settings_owned = {
        let lock = cc.config.lock().unwrap();
        lock.guild(guild_id).cloned()
    };

    let Some(ref mut settings) = settings_owned else {
        debug!("Nothing to clean up for guild: '{guild_id}'");
        return Ok(());
    };

    // TODO This may break in the future.

    // Clean up aliases.
    {
        let mut remove = Vec::new();

        for (k, v) in settings.aliases.iter() {
            let key = match v.split_whitespace().next() {
                Some(k) => k,
                None => {
                    remove.push(k.clone());
                    continue;
                },
            };

            if !(cc.chat_commands.list.contains_key(key) || settings.aliases.contains_key(key)) {
                remove.push(k.clone());
            }
        }

        info!("Cleaning up aliases: {}", remove.len());
        settings.aliases.retain(|k, _| !remove.contains(k));
    }

    // Clean up reaction-roles.
    {
        let mut remove = Vec::new();

        for key in settings.reaction_roles.keys() {
            let (left, right) = key.split_once('.').unwrap();
            let channel_id: Id<ChannelMarker> = left.parse().unwrap();
            let msg_id: Id<MessageMarker> = right.parse().unwrap();

            let ok = match cc.cache.message(msg_id) {
                Some(m) => m.channel_id() == channel_id,
                None => cc.http.message(channel_id, msg_id).send().await.is_ok(),
            };

            if !ok {
                remove.push(key.clone());
            }
        }

        info!("Cleaning up reaction-roles: {}", remove.len());
        settings.reaction_roles.retain(|k, _| !remove.contains(k));
    }

    // TODO Clean up perms.
    // TODO Clean up events.

    let mut lock = cc.config.lock().unwrap();
    let _ = lock.guild_mut(guild_id).map(|s| mem::swap(s, settings));

    lock.write_guild(guild_id)?;

    Ok(())
}
