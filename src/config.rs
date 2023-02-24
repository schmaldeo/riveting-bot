#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::prelude::*;
use std::mem;
use std::path::Path;

use serde::{Deserialize, Serialize};
use twilight_model::channel::message::ReactionType;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, MessageMarker, RoleMarker};
use twilight_model::id::Id;

use crate::commands::CommandError;
use crate::utils::prelude::*;
use crate::{config, parser, utils};

pub const CONFIG_FILE: &str = "./data/bot.json";
pub const GUILD_CONFIG_DIR: &str = "./data/guilds/";

/// Returns the default command prefix string.
fn default_prefix() -> String {
    String::from("!")
}

/// Returns a key which can be used to access reaction-roles mappings in `Settings`.
pub fn reaction_roles_key(channel_id: Id<ChannelMarker>, message_id: Id<MessageMarker>) -> String {
    format!("{channel_id}.{message_id}")
}

/// General settings for the bot.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Settings {
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub aliases: HashMap<String, String>,
    #[serde(default)]
    pub reaction_roles: HashMap<String, Vec<ReactionRole>>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            prefix: default_prefix(),
            aliases: HashMap::new(),
            reaction_roles: HashMap::new(),
        }
    }
}

/// Serializable bot configuration.
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Config {
    /// Global prefix.
    #[serde(default = "default_prefix")]
    pub prefix: String,

    /// Whitelisted guilds, disabled if `None`.
    #[serde(default)]
    pub whitelist: Option<HashSet<Id<GuildMarker>>>,

    /// Guild settings are serialized to separate files.
    #[serde(skip_serializing, default)]
    pub guilds: HashMap<Id<GuildMarker>, Settings>,
}

impl Config {
    /// Load the configuration file from `CONFIG_FILE`.
    pub fn load() -> AnyResult<Self> {
        info!("Loading config file");

        let mut cfg = String::new();
        {
            let mut config = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(CONFIG_FILE)?;

            config.read_to_string(&mut cfg)?;
        }

        match serde_json::from_str::<Self>(&cfg) {
            Ok(mut c) => {
                fs::create_dir_all(GUILD_CONFIG_DIR)
                    .map_err(|e| anyhow::anyhow!("Failed to create guilds dir: {}", e))?;

                c.load_guild_all()?;
                c.write()?; // Write back what was loaded.

                Ok(c)
            },
            Err(e) => {
                debug!("Could not load config: {}", e);
                info!("Creating a default config file");

                let def = Self::default();
                def.write()?; // Write the default config.

                Ok(def)
            },
        }
    }

    /// Force update `self` from file.
    pub fn reload(&mut self) -> AnyResult<()> {
        *self = Self::load()?;

        Ok(())
    }

    /// Write the configuration to a file in `CONFIG_FILE`.
    /// # Notes
    /// This will truncate and overwrite the file, any changes that are not in the new data will be lost.
    pub fn write(&self) -> AnyResult<()> {
        info!("Updating config file");

        let config = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(CONFIG_FILE)?;

        serde_json::to_writer_pretty(config, self)
            .map_err(|e| anyhow::anyhow!("Serialization error: {e}"))?;

        // Write guild configuration files.
        self.write_guild_all()?;

        Ok(())
    }

    /// Get guild's config.
    pub fn guild(&self, guild_id: Id<GuildMarker>) -> Option<&Settings> {
        self.guilds.get(&guild_id)
    }

    /// Get mutable reference to guild's config, if exists.
    pub fn guild_mut(&mut self, guild_id: Id<GuildMarker>) -> Option<&mut Settings> {
        self.guilds.get_mut(&guild_id)
    }

    /// Get mutable reference to guild's config. Creates default if not yet found.
    pub fn guild_or_default(&mut self, guild_id: Id<GuildMarker>) -> &mut Settings {
        self.guilds.entry(guild_id).or_default()
    }

    /// Set guild's custom prefix, returns previously set prefix.
    pub fn set_prefix(&mut self, guild_id: Id<GuildMarker>, prefix: &str) -> String {
        mem::replace(
            &mut self.guild_or_default(guild_id).prefix,
            prefix.to_string(),
        )
    }

    pub fn guild_or_global_prefix(&self, guild_id: Id<GuildMarker>) -> &str {
        self.guild(guild_id)
            .map(|s| &s.prefix)
            .unwrap_or(&self.prefix)
    }

    /// Add an alias, returns `Some(alias_command)` if it replaced one.
    pub fn add_alias(&mut self, guild_id: Id<GuildMarker>, alias: Alias) -> Option<String> {
        self.guild_or_default(guild_id)
            .aliases
            .insert(alias.name, alias.command)
    }

    /// Remove an alias, returns the removed alias value `Some(alias_command)` if successful.
    pub fn remove_alias(&mut self, guild_id: Id<GuildMarker>, alias_name: &str) -> Option<String> {
        self.guild_mut(guild_id)?.aliases.remove(alias_name)
    }

    /// Add a reaction-role configuration.
    pub fn add_reaction_roles(
        &mut self,
        guild_id: Id<GuildMarker>,
        channel_id: Id<ChannelMarker>,
        message_id: Id<MessageMarker>,
        map: Vec<ReactionRole>,
    ) {
        let key = config::reaction_roles_key(channel_id, message_id);

        self.guild_or_default(guild_id)
            .reaction_roles
            .insert(key, map);
    }

    /// Look up all guild configurations in `GUILD_CONFIG_DIR` and save them to `self`.
    pub fn load_guild_all(&mut self) -> AnyResult<()> {
        let paths = fs::read_dir(GUILD_CONFIG_DIR)?.flatten().map(|p| p.path());

        for path in paths {
            let name = match path.file_stem().and_then(|s| s.to_str()) {
                Some(name) => name,
                None => {
                    let path = path.display();
                    warn!("Invalid file name '{path}'");
                    continue;
                },
            };

            let id = match name.parse() {
                Ok(id) => id,
                Err(e) => {
                    let path = path.display();
                    warn!("Could not parse guild config file name '{path}': {e}");
                    continue;
                },
            };

            let content = match fs::read_to_string(&path) {
                Ok(content) => content,
                Err(e) => {
                    let path = path.display();
                    error!("Could not read guild config file '{path}': {e}");
                    continue;
                },
            };

            match serde_json::from_str::<Settings>(&content) {
                Ok(settings) => {
                    self.guilds.insert(id, settings);
                },
                Err(e) => {
                    let path = path.display();
                    error!("Could not deserialize guild config '{path}': {e}");
                },
            }
        }

        Ok(())
    }

    /// Look up a guild configuration in `GUILD_CONFIG_DIR` and save it to `self`.
    pub fn load_guild(&mut self, guild_id: Id<GuildMarker>) -> AnyResult<()> {
        let file_name = format!("{guild_id}.json");
        let path = Path::new(GUILD_CONFIG_DIR).join(file_name);
        let content = fs::read_to_string(path)?;
        let settings = serde_json::from_str::<Settings>(&content)?;

        self.guilds.insert(guild_id, settings);

        Ok(())
    }

    /// Save guild configurations in `self` to `GUILD_CONFIG_DIR`.
    pub fn write_guild_all(&self) -> AnyResult<()> {
        fs::create_dir_all(GUILD_CONFIG_DIR)
            .map_err(|e| anyhow::anyhow!("Failed to create guilds dir: {}", e))?;

        for (id, settings) in self.guilds.iter() {
            let file_name = format!("{id}.json");
            let path = Path::new(GUILD_CONFIG_DIR).join(file_name);

            let open = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path);

            let guild_config = match open {
                Ok(f) => f,
                Err(e) => {
                    let path = path.display();
                    error!("Could not write guild config file '{path}': {e}");
                    continue;
                },
            };

            serde_json::to_writer_pretty(guild_config, settings).unwrap_or_else(|e| {
                let path = path.display();
                error!("Could not serialize guild config '{path}': {e}");
            });
        }

        Ok(())
    }

    /// Save a guild configuration in `self` to `GUILD_CONFIG_DIR`.
    pub fn write_guild(&self, guild_id: Id<GuildMarker>) -> AnyResult<()> {
        fs::create_dir_all(GUILD_CONFIG_DIR)
            .map_err(|e| anyhow::anyhow!("Failed to create guilds dir: {}", e))?;

        let settings = self
            .guild(guild_id)
            .ok_or_else(|| anyhow::anyhow!("No custom configuration"))?;

        let file_name = format!("{guild_id}.json");

        let guild_config = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(Path::new(GUILD_CONFIG_DIR).join(file_name))?;

        serde_json::to_writer_pretty(guild_config, settings)
            .map_err(|e| anyhow::anyhow!("Serialization error: {e}"))?;

        Ok(())
    }
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
                "Spaces in an alias name are not supported: '{name}'",
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReactionRole {
    pub emoji: ReactionType,
    pub role: Id<RoleMarker>,
}

impl ReactionRole {
    pub const fn new(emoji: ReactionType, role: Id<RoleMarker>) -> Self {
        Self { emoji, role }
    }
}

impl PartialEq for ReactionRole {
    fn eq(&self, other: &Self) -> bool {
        utils::reaction_type_eq(&self.emoji, &other.emoji) && self.role == other.role
    }
}
