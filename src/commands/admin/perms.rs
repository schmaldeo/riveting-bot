use std::collections::HashSet;

use indoc::formatdoc;
use twilight_mention::parse::{MentionType, ParseMention};
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;

use crate::commands::{CommandAccess, CommandContext, CommandError, CommandResult};
use crate::utils::prelude::*;
use crate::{parser, utils};

/// Command: Manage permissions for bot's commands.
pub async fn perms(cc: CommandContext<'_>) -> CommandResult {
    if cc.msg.guild_id.is_none() {
        return Err(CommandError::Disabled);
    }

    cc.http
        .create_message(cc.msg.channel_id)
        .reply(cc.msg.id)
        .content(&formatdoc!(
            "```
            Command:
                {}
            Usage:
                perms allow <callables: command, alias> <targets: user, role, channel>
                perms deny <callables: command, alias> <targets: user, role, channel>
                perms clear <callables or targets: command, alias, user, role, channel>
            ```",
            cc.cmd.description,
        ))?
        .send()
        .await?;

    Err(CommandError::MissingArgs)
}

/// Command: List guild's permissions for bot's commands.
/// Usage: perms list
pub async fn list(cc: CommandContext<'_>) -> CommandResult {
    let Some(guild_id) = cc.msg.guild_id else {
        return Err(CommandError::Disabled);
    };

    let list = {
        let lock = cc.config.lock().unwrap();

        match lock.guild(guild_id) {
            Some(data) => format!("```json\n{}```", utils::pretty_nice_json(&data.perms)),
            None => String::new(),
        }
    };

    cc.http
        .create_message(cc.msg.channel_id)
        .reply(cc.msg.id)
        .content(&list)?
        .send()
        .await?;

    Ok(())
}

/// Command: Allow access to use command or alias.
/// Usage: perms allow <callables: command, alias> <targets: user, role, channel>
pub async fn allow(cc: CommandContext<'_>) -> CommandResult {
    let Some(guild_id) = cc.msg.guild_id else {
        return Err(CommandError::Disabled);
    };

    let args = parser::parse_args(cc.args.trim())?;
    let (callables, targets) = collect_parts(&cc, guild_id, args)?;

    if callables.is_empty() || targets.is_empty() {
        return Err(CommandError::MissingArgs);
    }

    let mut lock = cc.config.lock().unwrap();
    let perms = lock.guild_or_default(guild_id).perms_mut();

    debug!("Setting custom permissions to 'allowed' in {callables:?} for {targets:?}");

    for callable in callables {
        let perm = perms.entry(callable.to_string()).or_default();

        for target in targets.iter() {
            match target {
                CommandAccess::User(id) => perm.set_user(*id, true),
                CommandAccess::Role(id) => perm.set_role(*id, true),
                CommandAccess::Channel(id) => perm.enable_channel(*id).into(),
                _ => None,
            };
        }
    }

    // Update config file.
    lock.write()?;

    Ok(())
}

/// Command: Deny access to use command or alias.
/// Usage: perms deny <callables: command, alias> <targets: user, role, channel>
pub async fn deny(cc: CommandContext<'_>) -> CommandResult {
    let Some(guild_id) = cc.msg.guild_id else {
        return Err(CommandError::Disabled);
    };

    let args = parser::parse_args(cc.args.trim())?;
    let (callables, targets) = collect_parts(&cc, guild_id, args)?;

    if callables.is_empty() || targets.is_empty() {
        return Err(CommandError::MissingArgs);
    }

    let mut lock = cc.config.lock().unwrap();
    let perms = lock.guild_or_default(guild_id).perms_mut();

    debug!("Setting custom permissions to 'denied' in {callables:?} for {targets:?}");

    for callable in callables {
        let perm = perms.entry(callable.to_string()).or_default();

        for target in targets.iter() {
            match target {
                CommandAccess::User(id) => perm.set_user(*id, false),
                CommandAccess::Role(id) => perm.set_role(*id, false),
                CommandAccess::Channel(id) => perm.disable_channel(*id).into(),
                _ => None,
            };
        }
    }

    // Update config file.
    lock.write()?;

    Ok(())
}

/// Command: Clear rules for provided commands or aliases.
/// Usage: perms clear <callables or targets: command, alias, user, role, channel>
pub async fn clear(cc: CommandContext<'_>) -> CommandResult {
    let Some(guild_id) = cc.msg.guild_id else {
        return Err(CommandError::Disabled);
    };

    let args = parser::parse_args(cc.args.trim())?;

    let mut callables = HashSet::new();
    let mut targets = HashSet::new();

    for arg in args {
        if let Ok(mention) = MentionType::parse(arg) {
            // Convertion fails if the mention type is invalid.
            let target = mention.try_into()?;
            targets.insert(target);
        } else {
            callables.insert(arg);
        }
    }

    let mut lock = cc.config.lock().unwrap();

    let Some(guild) = lock.guild_mut(guild_id) else {
        debug!("No settings found for guild '{guild_id}'");
        return Ok(());
    };

    if callables.is_empty() && targets.is_empty() {
        return Err(CommandError::NotImplemented);
    } else if targets.is_empty() {
        debug!("Removing custom permissions for {callables:?}");
        // Remove all permissions from commands or aliases.
        for callable in callables {
            guild.perms_mut().remove(callable);
        }
    } else if callables.is_empty() {
        debug!("Removing custom permissions for {targets:?}");
        // For all commands or aliases remove each `target`.
        for perm in guild.perms_mut().values_mut() {
            for target in targets.iter() {
                match target {
                    CommandAccess::User(id) => perm.remove_user(*id),
                    CommandAccess::Role(id) => perm.remove_role(*id),
                    CommandAccess::Channel(id) => perm.enable_channel(*id).into(),
                    _ => None,
                };
            }
        }
    } else {
        debug!("Removing custom permissions in {callables:?} for {targets:?}");
        // Remove specific permissions from commands or aliases.
        for callable in callables {
            let Some(perm) = guild.perms_mut().get_mut(callable) else {
                continue;
            };

            for target in targets.iter() {
                match target {
                    CommandAccess::User(id) => perm.remove_user(*id),
                    CommandAccess::Role(id) => perm.remove_role(*id),
                    CommandAccess::Channel(id) => perm.enable_channel(*id).into(),
                    _ => None,
                };
            }
        }
    }

    // Update config file.
    lock.write()?;

    Ok(())
}

/// Returns `Ok((callables, targets))` if successful.
/// This will try to validate the presence of commands or aliases in `args`.
/// # Notes
/// This does not care about the order of arguments.
fn collect_parts<'a>(
    cc: &CommandContext<'_>,
    guild_id: Id<GuildMarker>,
    args: Vec<&'a str>,
) -> Result<(HashSet<&'a str>, HashSet<CommandAccess>), CommandError> {
    let mut callables = HashSet::new();
    let mut targets = HashSet::new();

    // For each `arg`, try to parse it as a user, role or channel;
    // Or try to find a matching command or alias.
    for arg in args {
        if let Ok(mention) = MentionType::parse(arg) {
            // Convertion fails if the mention type is invalid.
            let target = mention.try_into()?;
            targets.insert(target);
        } else {
            {
                let lock = cc.config.lock().unwrap();

                // Try to find a matching alias.
                if let Some(data) = lock.guild(guild_id) {
                    if data.aliases().contains_key(arg) {
                        callables.insert(arg);
                        continue; // Found a matching alias, go next.
                    }
                }
            }

            // Split the command into parts to check if a subcommand exists.
            let mut subs = parser::parse_args(arg)?.into_iter();
            let first = subs.next().unwrap();

            // Try to find a matching root command.
            let mut command = cc.chat_commands.list.get(first).ok_or_else(|| {
                CommandError::UnexpectedArgs(format!("Expected command or alias, got '{first}'"))
            })?;

            // Check for subcommands: All parts of the command must be in the subcommand tree.
            for sub in subs {
                command = command.sub_commands.get(sub).ok_or_else(|| {
                    CommandError::UnexpectedArgs(format!(
                        "Expected command, subcommand or alias, got '{sub}' in '{arg}'"
                    ))
                })?;
            }

            // Save the entire command.
            callables.insert(arg);
        }
    }

    Ok((callables, targets))
}
