use serde::{Deserialize, Serialize};

use crate::commands::{CommandContext, CommandError, CommandResult};
use crate::utils::*;

const DELIMITERS: &[char] = &['\'', '"', '`'];

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

    let alias = cc.args.parse::<Alias>()?;

    let mut lock = cc.config.lock().unwrap();

    lock.set_alias(guild_id, alias);
    lock.write()?;

    Ok(())
}
/// Command: Remove a guild command alias definition.
pub async fn remove(cc: CommandContext<'_>) -> CommandResult {
    let Some(guild_id) = cc.msg.guild_id else {
        return Err(CommandError::NotImplemented)
    };

    let args = cc.args.trim();

    let mut lock = cc.config.lock().unwrap();

    let alias_name = if is_surrounded_by(args, DELIMITERS)? {
        strip_delimits(args, DELIMITERS).ok_or(AliasError::InvalidPunctuation)?
    } else {
        args
    };

    lock.remove_alias(guild_id, alias_name);
    lock.write()?;

    Ok(())
}

pub enum AliasError {
    InvalidFormat,
    InvalidPunctuation,
}

impl std::fmt::Display for AliasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            AliasError::InvalidFormat => write!(f, "Alias format is invalid"),
            AliasError::InvalidPunctuation => write!(f, "Invalid punctuation in alias definition"),
        }
    }
}

impl From<AliasError> for CommandError {
    fn from(other: AliasError) -> Self {
        CommandError::UnexpectedArgs(other.to_string())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Alias {
    pub name: String,
    pub command: String,
}

impl std::str::FromStr for Alias {
    type Err = AliasError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // A rather simple parsing but I'm sure everything won't just explode, ok?

        let (left, right) = s.trim().split_once('=').ok_or(AliasError::InvalidFormat)?;
        let (left, right) = (left.trim(), right.trim());

        if !is_surrounded_by(left, DELIMITERS)? || !is_surrounded_by(right, DELIMITERS)? {
            return Err(AliasError::InvalidFormat);
        }

        let left = strip_delimits(left, DELIMITERS).ok_or(AliasError::InvalidPunctuation)?;
        let right = strip_delimits(right, DELIMITERS).ok_or(AliasError::InvalidPunctuation)?;

        Ok(Alias {
            name: left.to_string(),
            command: right.to_string(),
        })
    }
}

fn is_surrounded_by(target: &str, delimits: &[char]) -> Result<bool, AliasError> {
    let mut chars = target.chars();
    let left = chars.next().ok_or(AliasError::InvalidPunctuation)?;
    let right = chars.last().ok_or(AliasError::InvalidPunctuation)?;

    Ok(left == right && target.starts_with(delimits) && target.ends_with(delimits))
}

fn strip_delimits<'a>(s: &'a str, delimits: &[char]) -> Option<&'a str> {
    s.strip_prefix(delimits)
        .and_then(|s| s.strip_suffix(delimits))
}
