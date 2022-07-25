#![allow(dead_code)]

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::mem;
use std::pin::Pin;

use futures::Future;
use thiserror::Error;
use twilight_mention::parse::MentionType;
use twilight_model::channel::Message;
use twilight_model::gateway::payload::incoming::{ChannelUpdate, RoleUpdate};
use twilight_model::guild::Permissions;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, RoleMarker, UserMarker};
use twilight_model::id::Id;
use twilight_util::permission_calculator::PermissionCalculator;

use crate::utils::prelude::*;
use crate::Context;

/// Command builder functions.
pub mod builder;

/// Generic commands.
pub mod meta;

/// Normal user commands.
pub mod user;

/// Administrator comands.
#[cfg(feature = "admin")]
pub mod admin;

/// Bot owner only commands.
#[cfg(feature = "owner")]
pub mod owner;

pub type CommandResult = Result<(), CommandError>;

#[derive(Debug, Error)]
pub struct ClientError(#[from] twilight_http::Error);

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = &self.0;
        match err.kind() {
            twilight_http::error::ErrorType::Parsing { body }
            | twilight_http::error::ErrorType::Response { body, .. } => {
                write!(f, "{err}, body: {}", String::from_utf8_lossy(body))
            },
            _ => write!(f, "{err}"),
        }
    }
}

#[derive(Debug, Error)]
pub enum CommandError {
    /// A command prefix is needed.
    #[error("Message did not start with a command prefix")]
    NotPrefixed,

    /// A command does not exist.
    #[error("Command not found: {0}")]
    NotFound(String),

    /// A resource does not exist.
    #[error("Resource not found: {0}")]
    UnknownResource(String),

    /// Work-in-progress.
    #[error("Command or action not yet implemented")]
    NotImplemented,

    /// The sender must reply to a message.
    #[error("Expected reply reference missing")]
    MissingReply,

    /// The sender must provide some arguments.
    #[error("Expected arguments missing")]
    MissingArgs,

    /// Some arguments are wrong, invalid or unexpected.
    #[error("Arguments unexpected or failed to process: {0}")]
    UnexpectedArgs(String),

    /// Error while parsing command or argument.
    #[error("Failed to parse command or argument: {0}")]
    ParseError(String),

    /// The command or action is not available in this context.
    #[error("Command or action disabled")]
    Disabled,

    /// The sender does not have permissions needed.
    #[error("Permission requirements not met")]
    AccessDenied,

    /// Twilight client errors, when obscure.
    #[error(transparent)]
    Client(#[from] ClientError),

    /// Other errors that are or can be converted to `anyhow::Error`.
    #[error(transparent)]
    Other(#[from] anyhow::Error), // Source and Display delegate to `anyhow::Error`
}

impl PartialEq for CommandError {
    fn eq(&self, other: &Self) -> bool {
        mem::discriminant(self) == mem::discriminant(other) // Close enough.
    }
}

macro impl_into_command_error($out:ident; $t:ty) {
    impl From<$t> for CommandError {
        fn from(other: $t) -> Self {
            Self::$out(other.into())
        }
    }
}

impl_into_command_error!(Client; twilight_http::Error);
impl_into_command_error!(Other; twilight_http::response::DeserializeBodyError);
impl_into_command_error!(Other; twilight_validate::request::ValidationError);
impl_into_command_error!(Other; twilight_validate::message::MessageValidationError);
impl_into_command_error!(Other; twilight_standby::future::Canceled);
impl_into_command_error!(Other; serde_json::Error);
impl_into_command_error!(Other; std::fmt::Error);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CommandAccess {
    /// User with this id has access.
    User(Id<UserMarker>),

    /// Users with this role have access.
    Role(Id<RoleMarker>),

    /// Users on this channel have access.
    Channel(Id<ChannelMarker>),

    /// Users with these permissions have access.
    Permissions(Permissions),

    /// Bot owners have access.
    Owner,

    /// Available only in DMs.
    Dm,

    /// Anyone (who can send messages) has access.
    Any,
}

impl TryFrom<MentionType> for CommandAccess {
    type Error = CommandError;

    fn try_from(value: MentionType) -> Result<Self, Self::Error> {
        let expected = "Expected user, role or channel tag";

        match value {
            MentionType::User(id) => Ok(Self::User(id)),
            MentionType::Role(id) => Ok(Self::Role(id)),
            MentionType::Channel(id) => Ok(Self::Channel(id)),
            MentionType::Emoji(emoji) => Err(CommandError::UnexpectedArgs(format!(
                "{expected}, got emoji '{emoji}'"
            ))),
            MentionType::Timestamp(ts) => Err(CommandError::UnexpectedArgs(format!(
                "{expected}, got timestamp '{}'",
                ts.unix()
            ))),
            e => Err(CommandError::UnexpectedArgs(format!(
                "{expected}, got '{e}'"
            ))),
        }
    }
}

pub struct ChatCommands {
    pub list: BTreeMap<&'static str, Command>,
}

impl ChatCommands {
    pub fn new() -> Self {
        // Create the commands list and add commands to it.
        let mut list = BTreeMap::new();

        // Basic functionality.
        list.extend([
            command!(meta::ping).dm(true).desc("Ping the bot.").named(),
            command!(meta::about)
                .dm(true)
                .desc("Display bot info.")
                .named(),
            command!(meta::help)
                .dm(true)
                .desc("List bot commands.")
                .named(),
            command!(user::quote::quote)
                .desc("Manage quotes.")
                .sub(command!(user::quote::add))
                .sub(command!(user::quote::remove))
                .named(),
            command!(user::fuel::fuel)
                .dm(true)
                .desc("Calculate race fuel required.")
                .usage("fuel <length: minutes> <laptime: x:xx.xxx> <fuel per lap: x.xx>")
                .named(),
            command!(user::time::time)
                .dm(true)
                .desc("Display a discord timestamp.")
                .usage("time <date or time>")
                .named(),
            #[cfg(feature = "voice")]
            command!(user::voice::voice)
                .desc("Manage voice connection.")
                .sub(command!(user::voice::join))
                .sub(command!(user::voice::leave))
                .named(),
        ]);

        // Moderation functionality.
        #[cfg(feature = "admin")]
        list.extend([
            command!(admin; admin::config::config)
                .desc("Manage guild config.")
                .usage("cleanup")
                .usage("get")
                .usage("set <option> <value>")
                .sub(command!(admin; admin::config::cleanup))
                .sub(command!(admin; admin::config::get))
                .sub(command!(admin; admin::config::set))
                .named(),
            command!(admin; admin::alias::alias)
                .desc("Manage guild aliases.")
                .usage("list")
                .usage("get <name>")
                .usage("set <name> <definition>")
                .usage("remove <name>")
                .sub(command!(admin; admin::alias::list))
                .sub(command!(admin; admin::alias::get))
                .sub(command!(admin; admin::alias::set))
                .sub(command!(admin; admin::alias::remove))
                .named(),
            command!(admin; admin::perms::perms)
                .desc("Manage command and alias permissions.")
                .usage("list")
                .usage("allow <callables: command, alias> <targets: user, role, channel>")
                .usage("deny <callables: command, alias> <targets: user, role, channel>")
                .usage("clear <callables or targets: command, alias, user, role, channel>")
                .sub(command!(admin; admin::perms::list))
                .sub(command!(admin; admin::perms::allow))
                .sub(command!(admin; admin::perms::deny))
                .sub(command!(admin; admin::perms::clear))
                .named(),
            command!(admin; admin::roles::roles)
                .desc("Manage reaction-roles.")
                .usage("setup")
                .usage("edit (reply)")
                .sub(command!(admin; admin::roles::setup))
                .sub(command!(admin; admin::roles::edit))
                .named(),
            command!(admin; admin::bot::bot)
                .desc("Create or edit bot messages.")
                .usage("edit (reply) <text>")
                .usage("say <text>")
                .sub(command!(admin; admin::bot::edit))
                .sub(command!(admin; admin::bot::say))
                .named(),
            command!(admin; admin::silence::mute)
                .desc("Silence someone in voice channel.")
                .usage("mute <user>")
                .named(),
            command!(admin; admin::silence::timeout)
                .desc("Give someone a timeout.")
                .usage("timeout <user>")
                .named(),
            command!(admin; admin::scheduler::scheduler)
                .desc("Manage events.")
                .usage("add <name> <year> <month> <day> <hour> <minute> <second>")
                .usage("rm <event_id>")
                .sub(command!(admin; admin::scheduler::add))
                .sub(command!(admin; admin::scheduler::rm))
                .named(),
            #[cfg(feature = "bulk-delete")] // Extension of `admin` features.
            command!(admin; "delete-messages", admin::bulk::delete_messages).named(),
        ]);

        // Bot owner functionality.
        #[cfg(feature = "owner")]
        list.extend([command!(owner; owner::shutdown).named()]);

        Self { list }
    }

    pub async fn process(&self, ctx: &Context, msg: &Message) -> CommandResult {
        // Get text after a command prefix, return if not prefixed.
        let Some(stripped) = unprefix(ctx, msg) else {
            return Err(CommandError::NotPrefixed)
        };

        // Split the message by unicode whitespace.
        let (first, rest) = stripped
            .split_once(char::is_whitespace)
            .unwrap_or((stripped, ""));

        // Wrap `first` in a `Cow`, so that it doesn't have to allocate every command that is not an alias.
        let first = Cow::Borrowed(first);

        // FIXME This is kinda ugly right now, gotta try make it nicer.
        // Check if message is an alias, and unalias it.
        let (first, rest) = match unalias(ctx, msg, &first) {
            Some(alias) => {
                // Split the message by unicode whitespace.
                let (first, alias_rest) = alias
                    .split_once(char::is_whitespace)
                    .unwrap_or((&alias, ""));

                (
                    Cow::Owned(first.to_string()),
                    Cow::Owned([alias_rest, " ", rest].concat()),
                )
            },
            None => (first, Cow::Borrowed(rest)),
        };

        // Find the command.
        let Some(cmd) = self.list.get(first.as_ref()) else {
            return Err(CommandError::NotFound(format!("Unknown '{}'", first)))
        };

        debug!("Checking permissions for '{}'", msg.content);
        // Check that the message sender has sufficient permissions.
        // Checking permissions here means that the user must have access to at least the base command,
        // regardless of sub-command specific permissions.
        cmd.permission_check(ctx, msg).await?;

        // Run any pre-execution code, after permissions are ok.
        self.before(ctx, msg, cmd).await?;

        // Execute the command.
        let res = CommandContext::new(ctx, msg, rest.as_ref(), cmd)
            .execute()
            .await;

        // Check the command execution result and run any post-execution code.
        self.after(ctx, msg, cmd, res).await
    }

    /// Runs just before the command is executed.
    async fn before(&self, _ctx: &Context, msg: &Message, cmd: &Command) -> CommandResult {
        info!("Executing command '{}' by '{}'", cmd.name, msg.author.name);

        Ok(())
    }

    /// Runs just after the command is executed.
    async fn after(
        &self,
        _ctx: &Context,
        _msg: &Message,
        cmd: &Command,
        res: CommandResult,
    ) -> CommandResult {
        // In case of errors this will just return back to handlers, for now.
        res?;

        // Otherwise, all went well.
        debug!("Successfully executed '{}'!", cmd.name);

        Ok(())
    }

    fn commands_help(&self) -> String {
        // TODO This should be part of Command.
        fn recursive_help(cmd: &Command, tabs: usize) -> String {
            let mut text = String::new();
            let descs = 30 - (tabs * 4 + cmd.name.len());
            let indent = "\t".repeat(tabs);

            text.push_str(&indent);
            text.push_str(cmd.name);

            if !cmd.description.is_empty() {
                text.push_str(&" ".repeat(descs));
                text.push_str(" # ");
                text.push_str(cmd.description);
            }

            text.push('\n');

            if !cmd.sub_commands.is_empty() {
                for sub in cmd.sub_commands.values() {
                    text.push_str(&recursive_help(sub, tabs + 1));
                }
            }

            text
        }

        let mut text = String::new();

        for cmd in self.list.values() {
            text.push_str(&recursive_help(cmd, 1));
        }

        text
    }
}

impl std::fmt::Debug for ChatCommands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatCommands")
            .field("list", &self.list.keys())
            .finish()
    }
}

impl std::fmt::Display for ChatCommands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.commands_help())
    }
}

/// Get `Some(&str)` after prefix, or `None` otherwise. If the message is a DM, then any guild prefix will be accepted.
fn unprefix<'a>(ctx: &Context, msg: &'a Message) -> Option<&'a str> {
    match msg.guild_id {
        Some(guild_id) => {
            // In guilds.
            let lock = ctx.config.lock().unwrap();

            // See if guild has set a prefix, otherwise use global.

            let prefix = lock.guild_or_global_prefix(guild_id);
            msg.content.strip_prefix(prefix)
        },
        None => {
            // In DMs.
            let lock = ctx.config.lock().unwrap();

            // Try to use global prefix, if fails, try to use any guild prefix.
            let mut stripped = msg.content.strip_prefix(&lock.prefix);
            let mut guilds = lock.guilds.values();

            // While `stripped` is none, i.e. no prefix has matched, and there are still guild configs to check.
            while let (Some(guild), None) = (guilds.next(), stripped) {
                stripped = msg.content.strip_prefix(&guild.prefix);
            }

            stripped
        },
    }
}

/// If message was sent in a guild try to get the alias for `stripped`.
fn unalias<'a>(ctx: &'a Context, msg: &Message, stripped: &str) -> Option<Cow<'a, str>> {
    if let Some(guild_id) = msg.guild_id {
        let lock = ctx.config.lock().unwrap();
        let settings = lock.guild(guild_id)?;
        let found = settings.aliases.get(stripped)?;

        info!("Found alias '{found}' for '{stripped}'");

        return Some(Cow::Owned(found.to_string()));
    }

    None
}

#[derive(Debug, Clone)]
pub struct CommandContext<'a> {
    /// Base bot context. (auto-derefed)
    ctx: &'a Context,
    /// Received message.
    msg: &'a Message,
    /// Command or subcommand arguments.
    args: &'a str,
    /// Reference to the command that is being executed.
    cmd: &'a Command,
}

impl<'a> CommandContext<'a> {
    /// Create a new command context.
    pub const fn new(ctx: &'a Context, msg: &'a Message, args: &'a str, cmd: &'a Command) -> Self {
        Self {
            ctx,
            msg,
            args,
            cmd,
        }
    }

    /// Process and execute the command function in the context.
    pub async fn execute(mut self) -> CommandResult {
        let mut stack = Vec::new();
        let mut args = self.args.trim();

        // Walk args and find the last sub-command in the chain.
        loop {
            // Check for every arg that is split off by whitespace, then once more for the last bit.
            let (arg, rest) = args
                .split_once(|c: char| c.is_whitespace())
                .unwrap_or((args, ""));

            // Try to find a sub-command that matches `arg` from last (sub-)command.
            match stack.last().unwrap_or(&self.cmd).sub_commands.get(arg) {
                Some(sub) => {
                    args = rest.trim(); // Set unprocessed args to be processed next.
                    stack.push(sub); // Save this command to the stack and continue processing args.
                },
                None => break, // Nothing more to process.
            }
        }

        // Use last sub-command or the main command for execution.
        self.cmd = stack.last().unwrap_or(&self.cmd);

        // Only use args that are for this sub-command.
        self.args = args;

        // Check for sub-command permissions. Base command permissions should already be checked.
        if !stack.is_empty() {
            self.cmd.permission_check(self.ctx, self.msg).await?;
        }

        // Run the command function with context and stuff.
        (self.cmd.func)(self).await
    }

    /// Return currently active prefix, either global prefix or a guild specific one.
    pub fn active_prefix(&self, guild_id: Option<Id<GuildMarker>>) -> String {
        let lock = self.config.lock().unwrap();

        match guild_id {
            Some(guild_id) => match lock.guild(guild_id) {
                Some(data) => data.prefix.to_string(),
                None => lock.prefix.to_string(),
            },
            None => lock.prefix.to_string(),
        }
    }
}

// Slight abuse maybe, but it's convenient.
impl std::ops::Deref for CommandContext<'_> {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        self.ctx
    }
}

/// Helper macro to create a `Command` from an async function.
///
/// By default, the macro will use the the last identifier in the provided rust-path as the command name,
/// which should be the function to run on that command.
/// To use a different command name, specify it before the function.
///
/// You can use preset functions such as `Command::admin` and `Command::owner`
/// by specifying them first followed by a semicolon.
///
/// # Examples
/// ```rust
/// pub async fn foo(cc: CommandContext<'_>) -> CommandResult {
///     // ...
///     Ok(())
/// }
/// ```
///
/// ```rust
/// let c = command!(foo);                             // Command::new("foo", wrap!(foo))
/// let c = command!(baz::bar::foo);                   // Command::new("foo", wrap!(baz::bar::foo))
/// let c = command!("foo-baz", foo);                  // Command::new("foo-baz", wrap!(foo))
/// let c = command!(admin; foo);                      // Command::admin("foo", wrap!(foo))
/// let c = command!(owner; "foo-baz", foo);           // Command::owner("foo-baz", wrap!(foo))
/// ```
pub macro command {
    ($preset:ident; $name:expr, $func:expr) => {{
        Command::$preset($name, wrap!($func))
    }},
    ($name:expr, $func:expr) => {{
        Command::new($name, wrap!($func))
    }},
    ($preset:ident; $func:expr) => {{
        let name = name_from_rust_path(stringify!($func));
        Command::$preset(name, wrap!($func))
    }},
    ($func:expr) => {{
        let name = name_from_rust_path(stringify!($func));
        Command::new(name, wrap!($func))
    }},
}

fn name_from_rust_path(s: &str) -> &str {
    let out = s.rsplit("::").next().unwrap();

    if !out.chars().all(|c| c.is_alphanumeric() || c == '_') {
        panic!("Characters in the name should be alphanumeric or '_'");
    }

    out
}

/// Utility macro to wrap an async command function.
macro wrap($func:expr) {{
    pub fn wrapper(cmd: CommandContext) -> CommandFuture {
        Box::pin($func(cmd))
    }
    wrapper
}}

type CommandFuture<'a> = Pin<Box<dyn Future<Output = CommandResult> + Send + 'a>>;
type CommandFn = fn(CommandContext) -> CommandFuture;

/// Chat command or sub-command data, function and permissions.
pub struct Command {
    pub name: &'static str,
    pub func: CommandFn,
    pub description: &'static str,
    pub usage: Vec<&'static str>,
    pub sub_commands: BTreeMap<&'static str, Command>,
    pub access: CommandAccess,
    pub dm_enabled: bool,
}

impl_debug_struct_fields!(Command {
    name,
    description,
    usage,
    sub_commands,
    access,
    dm_enabled,
});

impl Command {
    /// Create a new command.
    pub fn new(name: &'static str, func: CommandFn) -> Self {
        debug_assert!(!name.is_empty(), "Command name should not be empty");

        Self {
            name,
            func,
            description: "",
            usage: Vec::new(),
            sub_commands: BTreeMap::new(),
            access: CommandAccess::Any,
            dm_enabled: false,
        }
    }

    /// Create a new command that only works in direct messages.
    pub fn dm_only(name: &'static str, func: CommandFn) -> Self {
        Self::new(name, func).access(CommandAccess::Dm).dm(true)
    }

    /// Create a new command with admin permissions.
    pub fn admin(name: &'static str, func: CommandFn) -> Self {
        Self::new(name, func).access(CommandAccess::Permissions(Permissions::ADMINISTRATOR))
    }

    /// Create a new command with bot owner permissions.
    pub fn owner(name: &'static str, func: CommandFn) -> Self {
        Self::new(name, func).access(CommandAccess::Owner).dm(true)
    }

    /// Returns `(self.name, self)` pair of the command.
    pub const fn named(self) -> (&'static str, Self) {
        (self.name, self)
    }

    /// Add a sub-command.
    pub fn sub(mut self, cmd: Self) -> Self {
        self.sub_commands.insert(cmd.name, cmd);
        self
    }

    /// Set guild permissions command access.
    pub const fn access(mut self, access: CommandAccess) -> Self {
        // TODO Inferred sub-command permissions.
        self.access = access;
        self
    }

    /// Set direct message access.
    pub const fn dm(mut self, enabled: bool) -> Self {
        self.dm_enabled = enabled;
        self
    }

    /// Set a description for the command.
    pub const fn desc(mut self, desc: &'static str) -> Self {
        self.description = desc;
        self
    }

    /// Set a usage for the command.
    pub fn usage(mut self, usage: &'static str) -> Self {
        self.usage.push(usage);
        self
    }

    /// Determine caller permissions access.
    pub async fn permission_check(&self, ctx: &Context, msg: &Message) -> CommandResult {
        match msg.guild_id {
            // No guild is present and DMs are disabled.
            None if !self.dm_enabled => Err(CommandError::Disabled),
            // Ignore guild permissions.
            None => Ok(()),
            // Check for guild permissions.
            Some(guild_id) => {
                match self.check_custom_permissions(ctx, msg, guild_id) {
                    Some(true) => return Ok(()),
                    Some(false) => return Err(CommandError::AccessDenied),
                    None => (), // Use defaults.
                }

                let access = match self.access {
                    CommandAccess::User(u) => u == msg.author.id,
                    CommandAccess::Role(r) => {
                        // TODO Might have to get roles from http client (and not unwrap).

                        // The user must have this role.
                        // Also check for `@everyone` role, which has the same id as the guild.
                        msg.member.as_ref().unwrap().roles.contains(&r) || r == guild_id.cast()
                    },
                    CommandAccess::Channel(c) => c == msg.channel_id,
                    CommandAccess::Permissions(p) => {
                        // The user must have these permissions.
                        sender_has_permissions(ctx, msg, guild_id, p).await?
                    },
                    CommandAccess::Owner => {
                        // The user is owner privileged.
                        let owner = match &ctx.application.owner {
                            Some(o) => o.id,
                            None => match &ctx.application.team {
                                Some(t) => t.owner_user_id,
                                None => panic!("No owners found"),
                            },
                        };

                        msg.author.id == owner
                    },
                    CommandAccess::Dm => return Err(CommandError::Disabled), // Not enabled in guilds.
                    CommandAccess::Any => true, // Don't care, didn't ask.
                };

                if access {
                    Ok(())
                } else {
                    Err(CommandError::AccessDenied)
                }
            },
        }
    }

    /// Check configuration for guild's custom permissions.
    fn check_custom_permissions(
        &self,
        ctx: &Context,
        msg: &Message,
        guild_id: Id<GuildMarker>,
    ) -> Option<bool> {
        let lock = ctx.config.lock().unwrap();
        let guild = lock.guild(guild_id)?;

        // FIXME This is sort of a quickfix to cope with the ever-so-naive command splitting.
        // Everything would explode if there was any subcommands involved.
        let prefix = lock.guild_or_global_prefix(guild_id);
        let stripped = msg.content.strip_prefix(prefix)?;
        let (first, _) = stripped
            .split_once(char::is_whitespace)
            .unwrap_or((stripped, ""));
        let is_alias = lock.guild(guild_id)?.aliases.contains_key(first);

        // Just yeet out of here.
        if first != self.name && !is_alias {
            return None;
        }

        let cmd = if is_alias { first } else { self.name };
        let perm = guild.perms.get(cmd)?;

        // If the command is disabled in the channel then access is denied.
        if perm.is_channel_disabled(msg.channel_id) {
            return Some(false);
        }

        // Is allowed or denied for user specifically?
        if let Some(val) = perm.user(msg.author.id) {
            return Some(val);
        }

        // Otherwise, check if user has any role with explicit rule.
        let mut explicit = false;

        for role in msg.member.as_ref().unwrap().roles.iter() {
            match perm.role(*role) {
                Some(false) => return Some(false), // Explicitly denied.
                Some(true) => explicit = true,     // Explicitly allowed.
                None => (),                        // Not specified.
            }
        }

        if explicit {
            return Some(true);
        }

        None // Not specified.
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Command: {}", self.description)?;

        if self.sub_commands.is_empty() {
            writeln!(f, "Usage: {} <args>", self.name)?;
        } else {
            writeln!(f, "Usage: {} <subcommand>", self.name)?;
        }

        for usage in self.usage.iter() {
            writeln!(f, "\t{}", usage)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct CommandCall<'a> {
    prefix: &'a str,
    cmds: Vec<&'a Command>,
    args: &'a str,
}

impl<'a> CommandCall<'a> {
    /// Create a new command call from parameters.
    pub fn new(prefix: &'a str, cmds: Vec<&'a Command>, args: &'a str) -> Self {
        Self { prefix, cmds, args }
    }

    /// Parse a commands and args from `text` in the context of `ctx` and `guild_id`.
    /// Returns `None` if no base command is found (the first *thing* after prefix),
    /// or none of the prefixes from the context were matched.
    pub fn parse_from(
        ctx: &'a Context,
        guild_id: Option<Id<GuildMarker>>,
        text: &'a str,
    ) -> Option<Self> {
        let prefixes = {
            let lock = ctx.config.lock().unwrap();

            match guild_id {
                // In a guild, use custom prefix, otherwise global default.
                Some(guild_id) => vec![lock
                    .guild(guild_id)
                    .map(|s| s.prefix.clone())
                    .unwrap_or_else(|| lock.prefix.clone())],

                // In a DM, global default or any guild prefix is accepted.
                None => {
                    let mut prefixes = [&lock.prefix]
                        .into_iter()
                        .chain(lock.guilds.values().map(|s| &s.prefix))
                        .filter(|s| !s.is_empty())
                        .cloned()
                        .collect::<Vec<_>>();

                    prefixes.sort_unstable();
                    prefixes.dedup();
                    prefixes
                },
            }
        };

        let (prefix, unprefixed) = unprefix_with(prefixes, text)?;

        // `args` is shortened in the loop as more commands are found.
        let (cmd, mut args) = unprefixed
            .split_once(char::is_whitespace)
            .unwrap_or((unprefixed, ""));

        // If no base command is found, all hope is lost.
        let base = ctx.chat_commands.list.get(cmd)?;

        let mut parsed = Self::new(prefix, vec![base], "");

        parsed.args = loop {
            // Split by whitespace, otherwise `args` is the last thing to check.
            let (cmd, rest) = args.split_once(char::is_whitespace).unwrap_or((args, ""));

            // `parsed` should always have at least one element.
            match parsed.cmds.last().and_then(|c| c.sub_commands.get(cmd)) {
                Some(cmd) => parsed.cmds.push(cmd), // Add last found command.
                None => break args, // No command found, rest of the string is arguments.
            }

            // To be checked next iteration.
            args = rest;
        };

        Some(parsed)
    }
}

impl std::fmt::Display for CommandCall<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.prefix)?;

        for cmd in self.cmds.iter() {
            write!(f, "{} ", cmd.name)?;
        }

        write!(f, "{}", self.args)
    }
}

/// Returns `Some(prefix, unprefixed)`,
/// where `prefix` is the matched prefix and `unprefixed` is everything after.
/// Otherwise, returns `None` if no prefix was matched from `prefixes`.
fn unprefix_with<I, T>(prefixes: I, text: &str) -> Option<(&str, &str)>
where
    I: IntoIterator<Item = T>,
    T: AsRef<str>,
{
    for prefix in prefixes {
        let prefix = prefix.as_ref();
        let stripped = text.strip_prefix(prefix);

        if let Some(stripped) = stripped {
            return Some((&text[..prefix.len()], stripped));
        }
    }

    None
}

/// Calculate if the message sender has `perms` permissions.
pub async fn sender_has_permissions(
    ctx: &Context,
    msg: &Message,
    guild_id: Id<GuildMarker>,
    perms: Permissions,
) -> AnyResult<bool> {
    // `@everyone` role id is the same as the guild's id.
    let everyone_id = guild_id.cast();

    // The member's assigned roles' ids + `@everyone` role id.
    let member_role_ids = msg
        .member
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?
        .roles
        .iter()
        .copied()
        .chain([everyone_id])
        .collect::<Vec<_>>();

    let mut iter = member_role_ids.iter();
    let mut cached_roles = Vec::with_capacity(member_role_ids.len());

    // Try get the member's roles from the cache.
    let cached = loop {
        match iter.next() {
            Some(id) => match ctx.cache.role(*id) {
                Some(r) => cached_roles.push(r.resource().to_owned()),
                None => break false,
            },
            None => break true,
        }
    };

    let roles = if cached {
        // All roles were found in the cache.
        debug!("Using cached roles for '{}'", msg.author.name);

        cached_roles
    } else {
        // Need to fetch the guild's roles from the http client.
        debug!("Fetching roles with http for '{}'", msg.author.name);

        let fetch = ctx.http.roles(guild_id).send().await?;

        // Filter only the roles that the member has.
        let member_roles = fetch
            .iter()
            .filter(|r| member_role_ids.contains(&r.id))
            .cloned()
            .collect();

        // Manually update the cache.
        for role in fetch {
            ctx.cache.update(&RoleUpdate { guild_id, role });
        }

        member_roles
    };

    // Permissions that are given by `@everyone` role
    let everyone_perm = roles
        .iter()
        .find(|r| r.id == everyone_id)
        .expect("'@everyone' role not found")
        .permissions;

    // Map roles into a `PermissionCalculator` happy format.
    let roles: Vec<_> = roles.into_iter().map(|r| (r.id, r.permissions)).collect();

    // Get the channel in which the message was sent.
    let channel = match ctx.cache.channel(msg.channel_id) {
        Some(chan) => {
            // Use cached channel.
            debug!(
                "Using cached channel for '{}'",
                chan.name
                    .as_ref()
                    .map(Into::into)
                    .unwrap_or_else(|| chan.id.to_string())
            );

            chan.to_owned()
        },
        None => {
            // Fetch channel from the http client.
            let chan = ctx.http.channel(msg.channel_id).send().await?;

            debug!(
                "Fetching channel with http for '{}'",
                chan.name
                    .as_ref()
                    .map(Into::into)
                    .unwrap_or_else(|| chan.id.to_string())
            );

            // Manually update the cache.
            ctx.cache.update(&ChannelUpdate(chan.clone()));

            chan
        },
    };

    // Get channel specific permission overwrites.
    let overwrites = channel.permission_overwrites.unwrap_or_default();

    // Create a calculator.
    let calc = PermissionCalculator::new(guild_id, msg.author.id, everyone_perm, &roles);

    Ok(calc.in_channel(channel.kind, &overwrites).contains(perms))
}

/// Calculate if the message sender has administrator permissions.
pub async fn sender_has_admin(
    ctx: &Context,
    msg: &Message,
    guild_id: Id<GuildMarker>,
) -> AnyResult<bool> {
    // `@everyone` role id is the same as the guild's id.
    let everyone_id = guild_id.cast();

    // The member's assigned roles' ids + `@everyone` role id.
    let member_role_ids = msg
        .member
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?
        .roles
        .iter()
        .copied()
        .chain([everyone_id])
        .collect::<Vec<_>>();

    let mut iter = member_role_ids.iter();
    let mut cached_roles = Vec::with_capacity(member_role_ids.len());

    // Try get the member's roles from the cache.
    let cached = loop {
        match iter.next() {
            Some(id) => match ctx.cache.role(*id) {
                Some(r) => cached_roles.push(r.resource().to_owned()),
                None => break false,
            },
            None => break true,
        }
    };

    let roles = if cached {
        // All roles were found in the cache.
        debug!("Using cached roles for '{}'", msg.author.name);

        cached_roles
    } else {
        // Need to fetch the guild's roles from the http client.
        debug!("Fetching roles with http for '{}'", msg.author.name);

        let fetch = ctx.http.roles(guild_id).send().await?;

        // Filter only the roles that the member has.
        let member_roles = fetch
            .iter()
            .filter(|r| member_role_ids.contains(&r.id))
            .cloned()
            .collect();

        // Manually update the cache.
        for role in fetch {
            ctx.cache.update(&RoleUpdate { guild_id, role });
        }

        member_roles
    };

    Ok(roles
        .into_iter()
        .any(|r| r.permissions.contains(Permissions::ADMINISTRATOR)))
}
