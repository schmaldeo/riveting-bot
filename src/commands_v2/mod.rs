// TODO: Implement response handling.
// TODO: Implement arguments handling.

/*
Interaction -> Command -> Function -> Execution -> Response -> InteractionMessage -> Delete
                                   |      |           |                 |
                                   |      v           v                 v
                                   +--- stuff    NormalMessage        Update
*

Command -> Classic Command + Args -> Function
ChatInput -> App Command + Args -> Function
Message -> App Command + Message -> Function
User -> App Command + User -> Function


wrap function handles the "environment"
-> decides response
exec function is the core effect of the command event
-> command result

classic: text-based, can reference another message
slash: semi-text-based, interaction, cannot reference another message directly
user: gui-based, interaction, no options, all data through received types
message: gui-based, interaction, no options, all data through received types
*/

use std::collections::{BTreeMap, HashSet};
use std::mem;

use thiserror::Error;

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
    pub use crate::commands_v2::function::Function;
    pub use crate::commands_v2::request::{
        ClassicRequest, MessageRequest, SlashRequest, UserRequest,
    };
    pub use crate::commands_v2::{Command, CommandError, CommandResult, Response};
    pub use crate::Context;
}

#[allow(unused_variables)]
pub trait Command {
    /// Type that is used to pass additional data to `Command::uber`.
    type Data: Default = ();

    /// Handle general command event. Called by default implementations.
    /// Additional data may be passed from original event.
    async fn uber(ctx: Context, data: Self::Data) -> CommandResult {
        Ok(Response::Clear)
    }

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
    UpdateMessage(String),
}

pub type CommandResult = Result<Response, CommandError>;
pub type Commands = BTreeMap<&'static str, BaseCommand>;

#[derive(Debug, Default, Clone)]
pub struct CommandsBuilder {
    list: Vec<BaseCommand>,
}

impl CommandsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind(&mut self, cmd: impl Into<BaseCommand>) -> &mut Self {
        self.list.push(cmd.into());
        self
    }

    pub fn validate(&self) -> AnyResult<()> {
        let mut set = HashSet::with_capacity(self.list.len());

        for cmd in self.list.iter() {
            // Ensure command itself is valid.
            cmd.validate()
                .with_context(|| cmd.command.name.to_string())?;

            // Ensure it doesn't overlap with other commands.
            anyhow::ensure!(
                set.insert(&cmd.command.name),
                "Duplicate command found: {}",
                cmd.command.name
            );
        }

        Ok(())
    }

    pub fn build(self) -> Commands {
        self.list.into_iter().map(|b| (b.command.name, b)).collect()
    }
}
