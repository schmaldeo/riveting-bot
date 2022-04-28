use std::collections::{HashMap, HashSet};
use std::pin::Pin;

use futures::Future;
use thiserror::Error;
use twilight_model::channel::Message;
use twilight_model::gateway::payload::incoming::{ChannelUpdate, RoleUpdate};
use twilight_model::guild::Permissions;
use twilight_model::id::marker::{GuildMarker, RoleMarker};
use twilight_model::id::Id;
use twilight_util::permission_calculator::PermissionCalculator;
use twilight_validate::message::MessageValidationError;
use twilight_validate::request::ValidationError;

use crate::utils::*;
use crate::Context;

/// Generic commands.
pub mod meta;

/// Normal user commands.
pub mod user;

/// Administrator comands.
#[cfg(feature = "admin-commands")]
pub mod admin;

/// Bot owner only commands.
#[cfg(feature = "owner-commands")]
pub mod owner;

pub type CommandResult = Result<(), CommandError>;

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Message did not start with a command prefix")]
    NotPrefixed,

    #[error("Command '{0}' not found")]
    NotFound(String),

    #[error("Command not yet implemented")]
    NotImplemented,

    #[error("Expected reply reference missing")]
    MissingReply,

    #[error("Expected arguments missing")]
    MissingArgs,

    #[error("Arguments unexpected or failed to process")]
    UnexpectedArgs,

    #[error("Command disabled")]
    Disabled,

    #[error("Permission requirements not met")]
    AccessDenied,

    #[error(transparent)]
    Other(#[from] anyhow::Error), // Source and Display delegate to `anyhow::Error`
}

macro impl_into_command_error($t:ty) {
    impl From<$t> for CommandError {
        fn from(other: $t) -> Self {
            Self::Other(other.into())
        }
    }
}

impl_into_command_error!(ValidationError);
impl_into_command_error!(MessageValidationError);
impl_into_command_error!(twilight_http::Error);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CommandAccess {
    /// Users with this role have access.
    Role(Id<RoleMarker>),

    /// Users with these permissions have access.
    Permissions(Permissions),

    /// Bot owners have access.
    Owner,

    /// Available only in DMs.
    Dm,

    /// Anyone (who can send messages) has access.
    Any,
}

pub struct ChatCommands {
    pub list: HashMap<&'static str, Command>,
}

impl ChatCommands {
    pub fn new() -> Self {
        // Create the commands list and add commands to it.
        let mut list = HashMap::new();

        // Basic functionality.
        list.extend([
            command!(meta::ping).dm(true).named(),
            command!(meta::about).dm(true).named(),
            command!(meta::help).dm(true).named(),
            command!(user::quote::quote)
                .sub(command!(user::quote::add))
                .sub(command!(user::quote::remove))
                .named(),
            command!(user::voice::voice)
                .sub(command!(user::voice::join))
                .sub(command!(user::voice::leave))
                .named(),
            command!(user::muter::muter)
                .sub(command!(user::muter::timeout))
                .named(),
        ]);

        // Moderation functionality.
        #[cfg(feature = "admin-commands")]
        list.extend([
            command!(admin; admin::roles::roles).named(),
            command!(admin; admin::config::config)
                .sub(command!(admin; admin::config::get))
                .sub(command!(admin; admin::config::set))
                .named(),
            command!(admin; admin::alias::alias)
                .sub(command!(admin; admin::alias::get))
                .sub(command!(admin; admin::alias::set))
                .named(),
        ]);

        #[cfg(feature = "bulk-delete")] // Separate from `admin-commands` feature, because yes.
        list.extend([command!(admin; "delete-messages", admin::delete_messages).named()]);

        // Bot owner functionality.
        #[cfg(feature = "owner-commands")]
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
            .split_once(|c: char| c.is_whitespace())
            .unwrap_or((stripped, ""));

        // Find the command.
        let Some(cmd) = self.list.get(first) else {
            return Err(CommandError::NotFound(first.to_string()))
        };

        debug!("Checking permissions for '{}'", msg.content);
        // Check that the message sender has sufficient permissions.
        // Checking permissions here means that the user must have access to at least the base command,
        // regardless of sub-command specific permissions.
        cmd.permission_check(ctx, msg).await?;

        // Run any pre-execution code, after permissions are ok.
        self.before(ctx, msg, cmd).await?;

        // Execute the command.
        let res = CommandContext::new(ctx, msg, rest, cmd).execute().await;

        // Check the command execution result and run any post-execution code.
        self.after(ctx, msg, cmd, res).await
    }

    /// Runs just before the command is executed.
    async fn before(&self, ctx: &Context, msg: &Message, cmd: &Command) -> CommandResult {
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
            let indent = "\t".repeat(tabs);
            text.push_str(&indent);
            text.push_str(cmd.name);
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
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
            let prefix = &lock.guilds.get(&guild_id).unwrap_or(&lock.global).prefix;
            msg.content.strip_prefix(prefix)
        },
        None => {
            // In DMs.
            let lock = ctx.config.lock().unwrap();

            // Try to use global prefix, if fails, try to use any guild prefix.
            let mut stripped = msg.content.strip_prefix(&lock.global.prefix);
            let mut guilds = lock.guilds.values();

            // While `stripped` is none, i.e. no prefix has matched, and there are still guild configs to check.
            while let (Some(guild), None) = (guilds.next(), stripped) {
                stripped = msg.content.strip_prefix(&guild.prefix);
            }

            stripped
        },
    }
}

#[derive(Debug, Clone)]
pub struct CommandContext<'a> {
    ctx: &'a Context,
    msg: &'a Message,
    args: &'a str,
    cmd: &'a Command, // Eh maybe?
}

impl<'a> CommandContext<'a> {
    /// Create a new command context.
    pub fn new(ctx: &'a Context, msg: &'a Message, args: &'a str, cmd: &'a Command) -> Self {
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
/// You can use presets such as `Command::admin` and `Command::owner`
/// by specifying them first followed by a colon.
///
/// # Examples
/// ```rust
/// async fn foo(ctx: &Context, msg: &Message, args: SplitWhitespace<'_>) -> CommandResult {
///     // ...
///     Ok(())
/// }
/// ```
///
/// ```rust
/// let c = command!(foo);                   // Command::new("foo", wrap!(foo))
/// let c = command!("foo-baz", foo);        // Command::new("foo-baz", wrap!(foo))
/// let c = command!(admin: foo);            // Command::admin("foo", wrap!(foo))
/// let c = command!(owner: "foo-baz", foo); // Command::owner("foo-baz", wrap!(foo))
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
    pub sub_commands: HashMap<&'static str, Command>,
    pub access: CommandAccess,
    pub dm_enabled: bool,
}

impl_debug_struct_fields!(Command, name, sub_commands, access, dm_enabled);

impl Command {
    /// Create a new command.
    pub fn new(name: &'static str, func: CommandFn) -> Self {
        debug_assert!(!name.is_empty(), "Command name should not be empty");

        Self {
            name,
            func,
            sub_commands: HashMap::new(),
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
    pub fn named(self) -> (&'static str, Self) {
        (self.name, self)
    }

    /// Add a sub-command.
    pub fn sub(mut self, cmd: Command) -> Self {
        self.sub_commands.insert(cmd.name, cmd);
        self
    }

    /// Set guild permissions command access.
    pub fn access(mut self, access: CommandAccess) -> Self {
        // TODO Inferred sub-command permissions.
        self.access = access;
        self
    }

    /// Set direct message access.
    pub fn dm(mut self, enabled: bool) -> Self {
        self.dm_enabled = enabled;
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
                let access = match self.access {
                    CommandAccess::Role(r) => {
                        // TODO Might have to get roles from http client (and not unwrap).

                        // The user must have this role.
                        // Also check for `@everyone` role, which has the same id as the guild.
                        msg.member.as_ref().unwrap().roles.contains(&r) || r == guild_id.cast()
                    },
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
}

/// Calculate if the message sender has `perms` permissions.
async fn sender_has_permissions(
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
        .unwrap()
        .roles
        .iter()
        .copied()
        .chain([everyone_id])
        .collect::<HashSet<_>>();

    // Try get the member's roles from the cache.
    let cached_roles = member_role_ids
        .iter()
        .flat_map(|r| ctx.cache.role(*r))
        .map(|r| r.resource().to_owned())
        .collect::<Vec<_>>();

    let roles = if member_role_ids.len() == cached_roles.len() {
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
