use std::collections::{HashMap, HashSet};
use std::str::SplitWhitespace;

use thiserror::Error;
use twilight_cache_inmemory::UpdateCache;
use twilight_model::channel::{ChannelType, Message};
use twilight_model::gateway::payload::incoming::RoleUpdate;
use twilight_model::guild::Permissions;
use twilight_util::permission_calculator::PermissionCalculator;

use crate::utils::*;
use crate::Context;

pub mod admin;

pub mod meta;

#[cfg(feature = "owner-commands")]
pub mod owner;

#[async_trait]
pub trait CommandFunction: Send + Sync {
    async fn execute(
        &self,
        ctx: &Context,
        msg: &Message,
        args: SplitWhitespace<'_>,
    ) -> AnyResult<()> {
        Err(ChatCommandError::NotImplemented.into())
    }

    fn boxed() -> Box<dyn CommandFunction>
    where
        Self: Sized + Default + 'static,
    {
        Box::new(Self::default())
    }
}

#[derive(Debug, Error)]
pub enum ChatCommandError {
    #[error("Message did not start with a command prefix")]
    NotPrefixed,

    #[error("Command '{0}' was not found")]
    NotFound(String),

    #[error("Command is not yet implemented")]
    NotImplemented,

    #[error("Expected arguments were missing")]
    MissingArgs,
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

        // #[cfg(feature = "bulk-delete")]
        list.extend([("delete-messages", admin::DeleteMessages::boxed())]);

        Self {
            prefix: prefix.to_string(),
            list,
        }
    }

    pub async fn process(&self, ctx: &Context, msg: &Message) -> AnyResult<()> {
        let Some(cmd) = msg.content.strip_prefix(&self.prefix) else {
            return Err(ChatCommandError::NotPrefixed.into())
        };

        let mut parts = cmd.split_whitespace();

        // Next thing after prefix, fail if nothing.
        let Some(cmd) = parts.next() else {
            return Err(ChatCommandError::NotFound(cmd.to_string()).into())
        };

        // Find the command.
        let Some(func) = self.list.get(cmd) else {
            return Err(ChatCommandError::NotFound(cmd.to_string()).into())
        };

        self.before(ctx, msg, cmd).await?;

        let res = func.execute(ctx, msg, parts).await;

        self.after(ctx, msg, cmd, res).await?;

        Ok(())
    }

    pub async fn before(&self, ctx: &Context, msg: &Message, cmd: &str) -> AnyResult<()> {
        info!("Command '{}' by '{}'", cmd, msg.author.name);

        if msg.guild_id.is_none() {
            unimplemented!("not yet implemented");
        }

        // FIXME This will break with DMs

        let guild_id = msg.guild_id.unwrap();

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

        // Get channel specific permission overwrites.
        let overwrites = match ctx.cache.channel(msg.channel_id) {
            Some(c) => c.permission_overwrites.to_owned(),
            None => {
                ctx.http
                    .channel(msg.channel_id)
                    .send()
                    .await?
                    .permission_overwrites
            }
        }
        .unwrap_or_default();

        // Create a permissions calculator.
        let calc = PermissionCalculator::new(guild_id, msg.author.id, everyone_perm, &roles);

        println!(
            "Admin?: {:?}",
            calc.in_channel(ChannelType::GuildText, &overwrites)
                .contains(Permissions::ADMINISTRATOR)
        );

        Ok(())
    }

    pub async fn after(
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
