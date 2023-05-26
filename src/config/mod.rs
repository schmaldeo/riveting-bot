#![allow(dead_code)]

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::prelude::*;
use std::path::Path;
use std::{any, mem};

use derive_more::{Deref, DerefMut};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use twilight_model::channel::message::ReactionType;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, MessageMarker, RoleMarker};
use twilight_model::id::Id;

use crate::commands::CommandError;
use crate::config::storage::{Directory, Storage};
use crate::utils::prelude::*;
use crate::{config, parser, utils};

pub mod storage;

pub const CONFIG_FILE: &str = "./data/bot.json";
pub const GUILD_CONFIG_DIR: &str = "./data/guilds/";

/// Returns a key which can be used to access reaction-roles mappings in `Settings`.
pub fn reaction_roles_key(channel_id: Id<ChannelMarker>, message_id: Id<MessageMarker>) -> String {
    format!("{channel_id}.{message_id}")
}

/// General settings for the bot.
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Settings {
    #[serde(default)]
    pub prefix: Prefix,
    #[serde(default)]
    pub aliases: HashMap<String, String>,
    #[serde(default)]
    pub reaction_roles: HashMap<String, Vec<ReactionRole>>,
}

/// Serializable bot configuration.
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Config {
    /// Global prefix.
    #[serde(default)]
    pub prefix: Prefix,

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

/// Bot classic command prefix.
#[derive(Deserialize, Serialize, Debug, Clone, Deref, DerefMut)]
pub struct Prefix(String);

impl Default for Prefix {
    fn default() -> Self {
        Self(String::from("!"))
    }
}

/// Serializable bot configuration.
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct BotSettings {
    /// Global classic command prefix.
    #[serde(default)]
    pub prefix: Prefix,

    /// Whitelisted guilds, disabled if `None`.
    #[serde(default)]
    pub whitelist: Option<HashSet<Id<GuildMarker>>>,
}

/// Custom data.
#[derive(Deserialize, Serialize, Debug, Default, Clone, Deref, DerefMut)]
pub struct Custom(HashMap<String, serde_json::Value>);

/// Error for when data does not match type.
#[derive(Debug, Error)]
#[error("Custom data with name '{name}' is not compatible with type '{ty_name}'")]
struct IncompatibleTypeError {
    ty_name: &'static str,
    name: String,
}

impl IncompatibleTypeError {
    fn new<T>(name: &str) -> Self {
        Self {
            ty_name: any::type_name::<T>(),
            name: name.to_string(),
        }
    }
}

/// Custom data entry guard.
#[derive(Debug)]
pub struct CustomEntry<'a> {
    dir: Directory<'a>,
}

impl<'a> CustomEntry<'a> {
    /// Create a new custom entry guard.
    pub const fn new(dir: Directory<'a>) -> Self {
        Self { dir }
    }

    /// Get custom data, if exists.
    pub fn get<T>(&self, name: &str) -> AnyResult<T>
    where
        T: DeserializeOwned,
    {
        self.dir
            .get::<Custom>()
            .with_context(|| format!("Custom data with name '{name}' is not loaded"))
            .and_then(|c| {
                c.get(name)
                    .with_context(|| format!("No custom data with name '{name}' found"))
            })
            .and_then(|v| serde_json::from_value(v.to_owned()).map_err(Into::into))
    }

    /// Force save custom data.
    pub fn overwrite<T>(&mut self, name: String, data: T) -> AnyResult<()>
    where
        T: Serialize,
    {
        self.save_with(|c| {
            let value = serde_json::to_value(data)?;
            c.insert(name, value);
            Ok(())
        })
    }

    /// Save custom data.
    ///
    /// # Errors
    /// If previously saved data is not compatible with the type.
    pub fn save<T>(&mut self, name: String, data: T) -> AnyResult<()>
    where
        T: Serialize + DeserializeOwned,
    {
        self.save_with(|c| {
            if let Some(value) = c.remove(&name) {
                serde_json::from_value::<T>(value)
                    .with_context(|| IncompatibleTypeError::new::<T>(&name))?;
            }
            let value = serde_json::to_value(data)?;
            c.insert(name, value);
            Ok(())
        })
    }

    /// Load custom data.
    pub fn load<T>(&mut self, name: &str) -> AnyResult<T>
    where
        T: DeserializeOwned,
    {
        self.dir
            .load::<Custom>()
            .with_context(|| format!("Could not load custom data with name '{name}'"))
            .and_then(|c| {
                c.get(name)
                    .with_context(|| format!("No custom data with name '{name}' found"))
            })
            .and_then(|v| {
                serde_json::from_value(v.to_owned())
                    .with_context(|| IncompatibleTypeError::new::<T>(name))
            })
    }

    /// Load custom data or create default.
    ///
    /// # Errors
    /// If loaded data is not compatible with the type.
    pub fn load_or_default<T>(&mut self, name: String) -> AnyResult<T>
    where
        T: Default + Serialize + DeserializeOwned,
    {
        let value = match self
            .dir
            .load_or_default_mut::<Custom>()?
            .entry(name.to_string())
        {
            Entry::Occupied(o) => o.get().to_owned(),
            Entry::Vacant(v) => {
                let value = serde_json::to_value(T::default())?;
                v.insert(value.to_owned());
                self.dir.save_from_memory::<Custom>()?;
                value
            },
        };
        serde_json::from_value(value).with_context(|| IncompatibleTypeError::new::<T>(&name))
    }

    /// Modify custom data with a function.
    fn save_with(&mut self, f: impl FnOnce(&mut Custom) -> AnyResult<()>) -> AnyResult<()> {
        self.dir.load_or_default_mut::<Custom>().and_then(f)?;
        self.dir.save_from_memory::<Custom>()
    }
}

#[derive(Debug)]
pub struct BotConfig {
    storage: Storage,
}

impl BotConfig {
    /// Setup a new configuration.
    pub fn new() -> AnyResult<Self> {
        let mut storage = Storage::default();

        storage.bind::<BotSettings>("bot")?;
        storage.bind::<Settings>("settings")?;
        storage.bind::<Custom>("custom")?;

        Ok(Self {
            storage: storage.validated()?,
        })
    }

    /// Return a reference to the inner storage type.
    pub const fn inner(&self) -> &Storage {
        &self.storage
    }

    /// Return classic command prefix, either global prefix or a guild specific one.
    pub fn classic_prefix(&self, guild_id: Option<Id<GuildMarker>>) -> AnyResult<Prefix> {
        let global_prefix = || {
            self.storage
                .global()
                .load_or_default::<BotSettings>()
                .map(|s| s.prefix.to_owned())
        };

        let guild_prefix = |guild_id| {
            self.storage
                .by_guild_id(guild_id)
                .load_or_default::<Settings>()
                .map(|s| s.prefix.to_owned())
                .or_else(|_| global_prefix())
        };

        guild_id.map_or_else(global_prefix, guild_prefix)
    }

    /// Save a reaction-role configuration.
    pub fn save_reaction_roles(
        &self,
        guild_id: Id<GuildMarker>,
        channel_id: Id<ChannelMarker>,
        message_id: Id<MessageMarker>,
        map: Vec<ReactionRole>,
    ) -> AnyResult<()> {
        let mut guild = self.storage.by_guild_id(guild_id);
        let settings = guild.load_or_default_mut::<Settings>()?;
        let key = config::reaction_roles_key(channel_id, message_id);
        settings.reaction_roles.insert(key, map);
        guild.save_from_memory::<Settings>()
    }

    /// Access custom data config.
    pub fn custom_entry(&self, guild_id: Option<Id<GuildMarker>>) -> CustomEntry {
        CustomEntry::new(self.directory(guild_id))
    }

    /// Returns global storage directory if `guild_id` is `None`,
    /// otherwise returns guild storage directory by guild id.
    fn directory(&self, guild_id: Option<Id<GuildMarker>>) -> Directory {
        guild_id.map_or_else(
            || self.storage.global(),
            |guild_id| self.storage.by_guild_id(guild_id),
        )
    }
}