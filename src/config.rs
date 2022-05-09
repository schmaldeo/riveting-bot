use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;

use crate::commands::admin::alias::Alias;
use crate::utils::prelude::*;

pub const CONFIG_PATH: &str = "./data/bot.json";

/// Set `Option` fields of `obj` of struct `target` to `Some(_)`
/// using either the set value in `obj` or `target::default()`.
/// `obj` must be an identifier of something mutable.
macro some_or_default {
    ($obj:ident: $target:ident { $($f:ident),*$(,)? }) => {{
        let def = $target::default();
        $( $obj.$f = $obj.$f.take().or(def.$f); )*
    }}
}

/// General settings for the bot.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Settings {
    pub prefix: Option<String>,
    pub aliases: Option<HashMap<String, String>>,
}

impl Settings {
    /// This will panic if set to `None`.
    pub fn prefix(&self) -> &str {
        self.prefix.as_ref().unwrap()
    }

    /// This will panic if set to `None`.
    pub fn aliases(&self) -> &HashMap<String, String> {
        self.aliases.as_ref().unwrap()
    }

    /// This will panic if set to `None`.
    pub fn prefix_mut(&mut self) -> &mut String {
        self.prefix.as_mut().unwrap()
    }

    /// This will panic if set to `None`.
    pub fn aliases_mut(&mut self) -> &mut HashMap<String, String> {
        self.aliases.as_mut().unwrap()
    }

    /// Make sure all settings are set to `Some(_)`.
    fn ensure_some_or_default(&mut self) {
        some_or_default!(self: Settings {
            prefix,
            aliases,
        });
    }
}

impl Default for Settings {
    /// Settings with `Some(_)` defaults.
    fn default() -> Self {
        Self {
            prefix: Some("!".to_string()),
            aliases: Some(HashMap::new()),
        }
    }
}

/// Serializable bot configuration.
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Config {
    pub global: Settings,
    pub guilds: HashMap<Id<GuildMarker>, Settings>,
}

impl Config {
    /// Load the configuration file from `CONFIG_PATH`.
    pub fn load() -> AnyResult<Config> {
        info!("Loading config file");

        let mut cfg = String::new();
        {
            let mut config = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(CONFIG_PATH)?;

            config.read_to_string(&mut cfg)?;
        }

        match serde_json::from_str::<Config>(&cfg) {
            Ok(c) => Ok(c.ensure_settings_or_default()), // Make sure loaded settings are at least something.
            Err(e) => {
                debug!("Could not load config: {}", e);
                info!("Creating a default config file");

                let def = Config::default();
                def.write()?;

                Ok(def)
            },
        }
    }

    /// Force update `self` from file.
    pub fn reload(&mut self) -> AnyResult<()> {
        *self = Self::load()?;

        Ok(())
    }

    /// Write the configuration to a file in `CONFIG_PATH`.
    /// # Notes
    /// This will truncate and overwrite the file, any changes that are not in the new data will be lost.
    pub fn write(&self) -> AnyResult<()> {
        info!("Updating config file");

        let config = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(CONFIG_PATH)?;

        serde_json::to_writer_pretty(config, self)?;

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

    /// Set guild's custom prefix, returns previously set prefix `Some(prefix)`.
    pub fn set_prefix(&mut self, guild_id: Id<GuildMarker>, prefix: &str) -> Option<String> {
        self.guild_or_default(guild_id)
            .prefix
            .replace(prefix.to_string())
    }

    /// Add an alias, returns `Some(alias_command)` if it replaced one.
    pub fn add_alias(&mut self, guild_id: Id<GuildMarker>, alias: Alias) -> Option<String> {
        self.guild_or_default(guild_id)
            .aliases_mut()
            .insert(alias.name, alias.command)
    }

    /// Remove an alias, returns the removed alias value `Some(alias_command)` if successful.
    pub fn remove_alias(&mut self, guild_id: Id<GuildMarker>, alias_name: &str) -> Option<String> {
        self.guild_mut(guild_id)?.aliases_mut().remove(alias_name)
    }

    /// Make sure all `Settings` in the config have fields set to `Some(_)`.
    fn ensure_settings_or_default(mut self) -> Self {
        self.global.ensure_some_or_default();
        self.guilds
            .iter_mut()
            .for_each(|(_, s)| s.ensure_some_or_default());
        self
    }
}

/// Thread-safe bot configuration wrapper.
#[derive(Debug, Clone)]
pub struct BotConfig(Arc<Mutex<Config>>);

impl BotConfig {
    /// Wrap a `Config` into a new `BotConfig(Arc<Mutex<Config>>)`.
    pub fn new(cfg: Config) -> Self {
        Self(Arc::new(Mutex::new(cfg)))
    }
}

impl std::ops::Deref for BotConfig {
    type Target = Arc<Mutex<Config>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
