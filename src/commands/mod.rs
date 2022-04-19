use std::collections::{HashMap, HashSet};
use std::str::SplitWhitespace;

use thiserror::Error;
use twilight_cache_inmemory::UpdateCache;
use twilight_model::channel::{ChannelType, Message};
use twilight_model::gateway::payload::incoming::{ChannelUpdate, RoleUpdate};
use twilight_model::guild::Permissions;
use twilight_model::id::marker::{GuildMarker, RoleMarker};
use twilight_model::id::Id;
use twilight_util::permission_calculator::PermissionCalculator;

use crate::utils::*;
use crate::Context;

pub mod admin;

pub mod meta;

#[cfg(feature = "owner-commands")]
pub mod owner;

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Message did not start with a command prefix")]
    NotPrefixed,

    #[error("Command '{0}' not found")]
    NotFound(String),

    #[error("Command not yet implemented")]
    NotImplemented,

    #[error("Expected arguments missing")]
    MissingArgs,

    #[error("Command disabled")]
    Disabled,

    #[error("Permission requirements not met")]
    AccessDenied,
}

#[derive(Debug)]
pub enum CommandAccess {
    /// Users with this role have access.
    Role(Id<RoleMarker>),

    /// Users with these permissions have access.
    Permissions(Permissions),

    /// Bot owners have access.
    Owner,

    /// Anyone (who can send messages) has access.
    Any,
}

#[async_trait]
pub trait CommandFunction: Send + Sync {
    /// Utility function to create a trait object.
    fn boxed() -> Box<dyn CommandFunction>
    where
        Self: Sized + Default + 'static,
    {
        Box::new(Self::default())
    }

    /// Returns `true` if the command can be used in DMs.
    /// By default, returns `false`.
    fn is_dm_enabled(&self) -> bool {
        false
    }

    /// Returns permissions or a role required for the command.
    /// By default, this returns full access, meaning no requirements are set.
    fn permissions(&self) -> AnyResult<CommandAccess> {
        Ok(CommandAccess::Any)
    }

    /// Function that runs when the command is called.
    async fn execute(
        &self,
        ctx: &Context,
        msg: &Message,
        args: SplitWhitespace<'_>,
    ) -> AnyResult<()> {
        Err(CommandError::NotImplemented.into())
    }
}

pub struct ChatCommands {
    pub prefix: String,
    pub list: HashMap<&'static str, Box<dyn CommandFunction>>,
}

impl ChatCommands {
    pub async fn new(prefix: &str) -> Self {
        #[allow(unused_mut)]
        let mut list = HashMap::from([
            ("ping", meta::Ping::boxed()),
            ("roles", admin::Roles::boxed()),
        ]);

        #[cfg(feature = "owner-commands")]
        list.extend([("shutdown", owner::Shutdown::boxed())]);

        #[cfg(feature = "bulk-delete")]
        list.extend([("delete-messages", admin::DeleteMessages::boxed())]);

        Self {
            prefix: prefix.to_string(),
            list,
        }
    }

    pub async fn process(&self, ctx: &Context, msg: &Message) -> AnyResult<()> {
        let Some(cmd) = msg.content.strip_prefix(&self.prefix) else {
            return Err(CommandError::NotPrefixed.into())
        };

        let mut parts = cmd.split_whitespace();

        // Next thing after prefix, fail if nothing.
        let Some(cmd) = parts.next() else {
            return Err(CommandError::NotFound(cmd.to_string()).into())
        };

        // Find the command.
        let Some(func) = self.list.get(cmd) else {
            return Err(CommandError::NotFound(cmd.to_string()).into())
        };

        // Check that the message sender has sufficient permissions.
        self.permission_check(ctx, msg, func.as_ref()).await?;

        // Run any pre-execution code, after permissions are ok.
        self.before(ctx, msg, cmd).await?;

        // Execute the command.
        let res = func.execute(ctx, msg, parts).await;

        // Check the command execution result and run any post-execution code.
        self.after(ctx, msg, cmd, res).await?;

        Ok(())
    }

    /// Determine caller permissions before going any further.
    async fn permission_check(
        &self,
        ctx: &Context,
        msg: &Message,
        func: &'_ dyn CommandFunction,
    ) -> AnyResult<()> {
        debug!("Checking permissions for '{}'", msg.content);

        match msg.guild_id {
            // No guild is present and DMs are disabled.
            None if !func.is_dm_enabled() => Err(CommandError::Disabled.into()),
            // Ignore guild permissions.
            None => Ok(()),
            // Check for guild permissions.
            Some(guild_id) => {
                let access = match func.permissions()? {
                    CommandAccess::Role(r) => {
                        // The user must have this role.
                        // Also check for `@everyone` role, which has the same id as the guild.
                        msg.member.as_ref().unwrap().roles.contains(&r) || r == guild_id.cast()
                    }
                    CommandAccess::Permissions(p) => {
                        // The user must have these permissions.
                        sender_has_permissions(ctx, msg, guild_id, p).await?
                    }
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
                    }
                    CommandAccess::Any => true, // Don't care, didn't ask.
                };

                if access {
                    Ok(())
                } else {
                    Err(CommandError::AccessDenied.into())
                }
            }
        }
    }

    /// Runs just before the command is executed.
    async fn before(&self, ctx: &Context, msg: &Message, cmd: &str) -> AnyResult<()> {
        info!("Executing command '{}' by '{}'", cmd, msg.author.name);

        Ok(())
    }

    /// Runs just after the command is executed.
    async fn after(
        &self,
        _ctx: &Context,
        _msg: &Message,
        cmd: &str,
        res: AnyResult<()>,
    ) -> AnyResult<()> {
        // Log if an error occurred.
        if let Err(e) = res {
            error!("Error in command '{}': {}", cmd, e);
        }

        Ok(())
    }
}

impl std::fmt::Debug for ChatCommands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatCommands")
            .field("prefix", &self.prefix)
            .field("list", &self.list.keys())
            .finish()
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
        }
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
        }
    };

    // Get channel specific permission overwrites.
    let overwrites = channel.permission_overwrites.unwrap_or_default();

    // Create a calculator.
    let calc = PermissionCalculator::new(guild_id, msg.author.id, everyone_perm, &roles);

    Ok(calc.in_channel(channel.kind, &overwrites).contains(perms))
}

/// `CommandFunction` shorthand for enabling DMs.
macro dm_enabled() {
    fn is_dm_enabled(&self) -> bool {
        true
    }
}

/// `CommandFunction` shorthand for admin permissions.
macro admin_permissions() {
    fn permissions(&self) -> AnyResult<CommandAccess> {
        Ok(CommandAccess::Permissions(Permissions::ADMINISTRATOR))
    }
}

/// `CommandFunction` shorthand for owner permissions.
macro owner_permissions() {
    fn permissions(&self) -> AnyResult<CommandAccess> {
        Ok(CommandAccess::Owner)
    }
}

// pub fn foo() -> Vec<Command> {
//     use twilight_model::application::command::{Command, CommandType};
//     use twilight_util::builder;
//     use twilight_util::builder::command::{BooleanBuilder, StringBuilder};
//     let c = builder::command::CommandBuilder::new(
//         "name".into(),
//         "description".into(),
//         CommandType::ChatInput,
//     )
//     .option(
//         StringBuilder::new("animal".into(), "The type of animal".into())
//             .required(true)
//             .choices([
//                 ("Dog".into(), "animal_dog".into()),
//                 ("Cat".into(), "animal_cat".into()),
//                 ("Penguin".into(), "animal_penguin".into()),
//             ]),
//     )
//     .option(BooleanBuilder::new(
//         "only_smol".into(),
//         "Whether to show only baby animals".into(),
//     ))
//     .validate()
//     .unwrap()
//     .build();
//     vec![c]
// }
