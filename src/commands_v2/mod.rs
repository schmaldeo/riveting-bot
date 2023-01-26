//! Command types:
//! - Classic: Chat text message, can reference another message.
//! - Slash: Semi-text-based interaction, cannot reference another message directly.
//! - Message: GUI-based interaction, no options, some data may be resolved by Discord or Twilight.
//! - User: GUI-based interaction, no options, some data may be resolved by Discord or Twilight.
//!
//! ```text
//!     ┌─────────────────┐           ┌─────────────┐       ┌──────────────┐
//!     │Interaction Event│        ┌─►│Slash Command├────┬─►│Create Request│
//!     └──────────┬──────┘        │  └─────────────┘    │  └───────┬──────┘
//!                │               │                     │          │
//!                │               │  ┌───────────────┐  │          ▼
//!                ▼               ├─►│Message Command├──┤  ┌──────────────────────────┐
//!         ┌───────────────────┐  │  └───────────────┘  │  │Execute Attached Functions│
//!         │Application Command├──┤                     │  └───────┬──────────────────┘
//!         └───────────────────┘  │  ┌────────────┐     │          │
//!                                └─►│User Command├─────┤          ▼
//!                                   └────────────┘     │  ┌─────────────────────┐
//!                                                      │  │Handle Command Result│
//!     ┌──────────────────┐          ┌───────────────┐  │  └───────┬─────────────┘
//!     │Chat Message Event├─────────►│Classic Command├──┘          │
//!     └──────────────────┘          └───────────────┘             │
//!                                                                 │
//!                                       ┌────────────────────┐    │
//!                                    ┌──┤Interaction Response│◄───┤
//!         ┌───────────────────────┐  │  └────────────────────┘    │
//!         │ Clear / Update / None │◄─┤                            │
//!         └───────────────────────┘  │  ┌─────────────────────┐   │
//!                                    └──┤Original Chat Message│◄──┘
//!                                       └─────────────────────┘
//! ```
//!

use std::collections::{BTreeMap, HashSet};
use std::mem;
use std::sync::Arc;

use derive_more::{Index, IntoIterator};
use thiserror::Error;

use crate::commands_v2::builder::twilight::{CommandValidationError, TwilightCommand};
use crate::commands_v2::builder::BaseCommand;
use crate::commands_v2::request::{ClassicRequest, MessageRequest, SlashRequest, UserRequest};
use crate::utils::prelude::*;
use crate::Context;

pub mod arg;
pub mod bot;
pub mod builder;
pub mod function;
pub mod handle;
pub mod request;

/// Prelude module for command things.
pub mod prelude {
    pub use crate::commands_v2::arg::{ArgValueExt, Args};
    pub use crate::commands_v2::request::{
        ClassicRequest, MessageRequest, SlashRequest, UserRequest,
    };
    pub use crate::commands_v2::{Command, CommandError, CommandResult, Response};
    pub use crate::Context;
}

/// A trait for defining functions for command executions and types.
/// This trait is purely used as a convenience "template" and so is not a must.
#[allow(unused_variables)]
pub trait Command {
    /// Type that is used to pass additional data to `Command::uber`.
    type Data: Default = ();

    // NOTE: At the time of writing, there is unusual behaviour with `feature(async_fn_in_trait)`
    // when calling default async impls of the trait. Therefore, implementors and users should not
    // trust functions that are not specifically implemented. When this problem is fixed,
    // `uber` should have a default implementation.
    // REVIEW: https://github.com/rust-lang/rust/issues/107002

    /// Handle general command event. Called by default implementations.
    /// Additional data may be passed from original event.
    async fn uber(ctx: Context, data: Self::Data) -> CommandResult;

    /// Called on classic command event. Default impl redirects to `Command::uber` with default data.
    async fn classic(ctx: Context, req: ClassicRequest) -> CommandResult {
        Self::uber(ctx, Default::default()).await
    }

    /// Called on slash command event. Default impl redirects to `Command::uber` with default data.
    async fn slash(ctx: Context, req: SlashRequest) -> CommandResult {
        Self::uber(ctx, Default::default()).await
    }

    /// Called on message command event. Default impl redirects to `Command::uber` with default data.
    async fn message(ctx: Context, req: MessageRequest) -> CommandResult {
        Self::uber(ctx, Default::default()).await
    }

    /// Called on user command event. Default impl redirects to `Command::uber` with default data.
    async fn user(ctx: Context, req: UserRequest) -> CommandResult {
        Self::uber(ctx, Default::default()).await
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

    /// Other errors that are or can be converted to `anyhow::Error`.
    #[error(transparent)]
    Other(#[from] anyhow::Error), // Source and Display delegate to `anyhow::Error`
}

impl PartialEq for CommandError {
    fn eq(&self, other: &Self) -> bool {
        mem::discriminant(self) == mem::discriminant(other) // Close enough.
    }
}

impl From<&'static str> for CommandError {
    fn from(s: &'static str) -> Self {
        Self::Other(anyhow::anyhow!(s))
    }
}

macro impl_into_command_error($out:ident; $t:ty) {
    impl From<$t> for CommandError {
        fn from(other: $t) -> Self {
            Self::$out(other.into())
        }
    }
}

impl_into_command_error!(Other; twilight_http::Error);
impl_into_command_error!(Other; twilight_http::response::DeserializeBodyError);
impl_into_command_error!(Other; twilight_validate::request::ValidationError);
impl_into_command_error!(Other; twilight_validate::message::MessageValidationError);
impl_into_command_error!(Other; twilight_standby::future::Canceled);
impl_into_command_error!(Other; serde_json::Error);
impl_into_command_error!(Other; std::fmt::Error);

#[derive(Debug, Clone)]
pub enum Response {
    None,
    Clear,
    CreateMessage(String),
}

pub type CommandResult = Result<Response, CommandError>;

/// Newtype for commands collection.
#[derive(Debug, Default, Clone, IntoIterator, Index)]
pub struct Commands(BTreeMap<&'static str, Arc<BaseCommand>>);

impl Commands {
    /// Get base command by name.
    pub fn get(&self, id: &str) -> Option<&Arc<BaseCommand>> {
        self.0.get(id)
    }

    /// Convert commands to Discord compatible list.
    pub fn twilight_commands(&self) -> Result<Vec<TwilightCommand>, CommandValidationError> {
        self.0
            .values()
            .flat_map(|b| b.twilight_commands())
            .try_collect()
    }

    /// Get reference to the inner list.
    pub const fn inner(&self) -> &BTreeMap<&'static str, Arc<BaseCommand>> {
        &self.0
    }
}

impl std::fmt::Display for Commands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0.keys().collect::<Vec<_>>()) // TODO: Display nice list
    }
}

/// A type for creating a collection of commands and validating them.
#[derive(Debug, Default, Clone)]
pub struct CommandsBuilder {
    list: Vec<BaseCommand>,
}

impl CommandsBuilder {
    /// Create a new list of commands.
    pub fn new() -> Self {
        Self::default()
    }

    /// Assign a command to the list.
    pub fn bind(&mut self, cmd: impl Into<BaseCommand>) -> &mut Self {
        self.list.push(cmd.into());
        self
    }

    /// Validate the list of commands.
    pub fn validate(&self) -> AnyResult<()> {
        let mut set = HashSet::with_capacity(self.list.len());

        for cmd in self.list.iter() {
            // Ensure command itself is valid.
            cmd.validate()?;

            // Ensure it doesn't overlap with other commands.
            anyhow::ensure!(
                set.insert(&cmd.command.name),
                "Duplicate command found: {}",
                cmd.command.name
            );
        }

        Ok(())
    }

    /// Finalize the list of commands.
    pub fn build(self) -> Commands {
        Commands(
            self.list
                .into_iter()
                .map(|b| (b.command.name, Arc::new(b)))
                .collect(),
        )
    }
}
