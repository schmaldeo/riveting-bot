use serde::{Deserialize, Serialize};

use crate::commands::{CommandContext, CommandError, CommandResult};
use crate::parser;
use crate::utils::prelude::*;
use crate::utils::{self, consts};

/// Command: Manage guild command aliases.
pub async fn alias(cc: CommandContext<'_>) -> CommandResult {
    Err(CommandError::NotImplemented)
}

/// Command: List guild command aliases.
pub async fn list(cc: CommandContext<'_>) -> CommandResult {
    let Some(guild_id) = cc.msg.guild_id else {
        return Err(CommandError::Disabled)
    };

    let list = {
        let lock = cc.config.lock().unwrap();

        match lock.guilds.get(&guild_id) {
            Some(data) => format!("```json\n{:#?}```", data.aliases), // Quick haxx
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

/// Command: Get a guild command alias definition.
pub async fn get(cc: CommandContext<'_>) -> CommandResult {
    Err(CommandError::NotImplemented)
}

/// Command: Set a guild command alias definition.
pub async fn set(cc: CommandContext<'_>) -> CommandResult {
    let Some(guild_id) = cc.msg.guild_id else {
        return Err(CommandError::NotImplemented)
    };

    let alias = cc.args.parse::<Alias>()?;

    let mut lock = cc.config.lock().unwrap();

    match lock.set_alias(guild_id, alias) {
        Some(_) => debug!("Replaced an alias in guild '{guild_id}'"),
        None => debug!("Added an alias in guild '{guild_id}'"),
    }
    lock.write()?;

    Ok(())
}

/// Command: Remove a guild command alias definition.
pub async fn remove(cc: CommandContext<'_>) -> CommandResult {
    let Some(guild_id) = cc.msg.guild_id else {
        return Err(CommandError::NotImplemented)
    };

    let (name, rest) = parser::maybe_quoted_arg(cc.args.trim())?;

    parser::ensure_rest_is_empty(rest)?;

    let mut lock = cc.config.lock().unwrap();

    match lock.remove_alias(guild_id, name) {
        Some(_) => {
            debug!("Removed an alias in guild '{guild_id}'");
            lock.write()?;
        },
        None => {
            return Err(CommandError::UnknownResource(format!(
                "Could not remove a nonexistent alias '{name}'"
            )))
        },
    }

    Ok(())
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Alias {
    pub name: String,
    pub command: String,
}

impl std::str::FromStr for Alias {
    type Err = CommandError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let args = s.trim();

        // Parse first arg as the name part.
        let (name, rest) = parser::maybe_quoted_arg(args)?;

        // Spaces in names are not supported, at least not yet.
        if name.contains(char::is_whitespace) {
            return Err(CommandError::UnexpectedArgs(format!(
                "Spaces in an alias name are not supported: '{}'",
                name
            )));
        }

        // Parse next arg as the command part.
        let (command, rest) = match rest {
            Some(command) => parser::maybe_quoted_arg(command)?,
            None => return Err(CommandError::MissingArgs),
        };

        parser::ensure_rest_is_empty(rest)?;

        Ok(Self {
            name: name.to_string(),
            command: command.to_string(),
        })
    }
}
