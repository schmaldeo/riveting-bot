//! # Overview
//!
//! ### Base command creation:
//! ```
//! macro command!(function, description) -> MappedCommand
//! macro command!(function, name, description) -> MappedCommand
//! ```
//!
//! ### Subcommand creation:
//! ```
//! macro func!(function, description) -> FunctionCommandBuilder
//! macro func!(function, name, description) -> FunctionCommandBuilder
//! ```
//!
//! ### Re-used twilight builders:
//! ```
//! fn attachment(name, description) -> AttachmentBuilder
//! fn boolean(name, description) -> BooleanBuilder
//! fn mention(name, description) -> ChannelBuilder
//! fn role(name, description) -> IntegerBuilder
//! fn user(name, description) -> MentionableBuilder
//! fn channel(name, description) -> NumberBuilder
//! fn integer(name, description) -> RoleBuilder
//! fn number(name, description) -> StringBuilder
//! fn string(name, description) -> UserBuilder
//! ```
//!

use twilight_model::application::command::{
    Command, CommandOption, CommandType, Number, OptionsCommandOptionData,
};
use twilight_model::guild::Permissions;
use twilight_model::id::Id;
use twilight_util::builder::command::{
    AttachmentBuilder, BooleanBuilder, ChannelBuilder, CommandBuilder, IntegerBuilder,
    MentionableBuilder, NumberBuilder, RoleBuilder, StringBuilder, UserBuilder,
};

pub use self::traits::*;
use crate::commands::{wrap, CommandFn};
use crate::utils::prelude::*;

mod traits {
    use twilight_model::application::command::{Command, Number};

    use crate::utils::prelude::*;

    pub trait IntegerBuilderExt {
        fn into_choices(
            self,
            choices: impl IntoIterator<Item = (impl ToString, impl Into<i64>)>,
        ) -> Self;
    }

    pub trait NumberBuilderExt {
        fn into_choices(
            self,
            choices: impl IntoIterator<Item = (impl ToString, impl Into<Number>)>,
        ) -> Self;
    }

    pub trait StringBuilderExt {
        fn into_choices(
            self,
            choices: impl IntoIterator<Item = (impl ToString, impl ToString)>,
        ) -> Self;
    }

    pub trait CommandBuilderExt<Output = Command>: Sized {
        fn build_checked(self) -> AnyResult<Output>;
    }
}

impl IntegerBuilderExt for IntegerBuilder {
    fn into_choices(
        self,
        choices: impl IntoIterator<Item = (impl ToString, impl Into<i64>)>,
    ) -> Self {
        self.choices(choices.into_iter().map(|(a, b)| (a.to_string(), b.into())))
    }
}

impl NumberBuilderExt for NumberBuilder {
    fn into_choices(
        self,
        choices: impl IntoIterator<Item = (impl ToString, impl Into<Number>)>,
    ) -> Self {
        self.choices(
            choices
                .into_iter()
                .map(|(a, b)| (a.to_string(), b.into().into())),
        )
    }
}

impl StringBuilderExt for StringBuilder {
    fn into_choices(
        self,
        choices: impl IntoIterator<Item = (impl ToString, impl ToString)>,
    ) -> Self {
        self.choices(
            choices
                .into_iter()
                .map(|(a, b)| (a.to_string(), b.to_string())),
        )
    }
}

impl CommandBuilderExt for CommandBuilder {
    fn build_checked(self) -> AnyResult<Command> {
        let cmd = self.build();
        validate_command(&cmd)?;
        Ok(cmd)
    }
}

/// Validates options in the command.
pub fn validate_command(cmd: &Command) -> AnyResult<()> {
    // Check with twilight's validation.
    twilight_validate::command::command(cmd)?;

    // Check for mutually exclusive options.
    let (mut sub, mut other) = (false, false);

    for opt in cmd.options.iter() {
        match opt {
            CommandOption::SubCommand(_) | CommandOption::SubCommandGroup(_) => sub |= true,
            _ => other |= true,
        }
    }

    anyhow::ensure!(
        !(sub && other),
        "Subcommands and subcommand groups are mutually exclusive with other options"
    );

    // Check for options ordering.
    fn check_ordering(opts: &[CommandOption]) -> AnyResult<()> {
        opts.iter()
            .map(CommandOption::is_required)
            .try_reduce(|a, b| {
                if a || !b {
                    Ok(b)
                } else {
                    anyhow::bail!("All required command options must be before any optional ones")
                }
            })
            .map(|_| ())?;

        for opt in opts.iter() {
            if let CommandOption::SubCommand(d) | CommandOption::SubCommandGroup(d) = opt {
                check_ordering(&d.options)?;
            }
        }

        Ok(())
    }

    check_ordering(&cmd.options)?;

    Ok(())
}

/// Creates shortcut functions for useful builders.
macro short_fn($v:vis fn $f:ident -> $t:ty) {
    /// Creates an argument builder.
    $v fn $f(name: impl ToString, description: impl ToString) -> $t {
        <$t>::new(name.to_string(), description.to_string())
    }
}

short_fn!(pub fn attachment -> AttachmentBuilder);
short_fn!(pub fn boolean -> BooleanBuilder);
short_fn!(pub fn mention -> MentionableBuilder);
short_fn!(pub fn role -> RoleBuilder);
short_fn!(pub fn user -> UserBuilder);
short_fn!(pub fn channel -> ChannelBuilder);
short_fn!(pub fn integer -> IntegerBuilder);
short_fn!(pub fn number -> NumberBuilder);
short_fn!(pub fn string -> StringBuilder);

/// Macro for making a function-command mapping builder with the provided name, description and a function.
/// This is meant for creation of subcommands.
///
/// Example:
/// ```
/// sub!(path::to::command, "description")
/// // Equivalent to:
/// // sub!(path::to::command, "command", "description")
/// ```
/// expands to
/// ```
/// FunctionCommandBuilder::new(wrap!(path::to::command), "command", "description")
/// ```
pub macro func {
    ($function:expr, $name:expr, $description:expr) => {{
        FunctionCommandBuilder::new(wrap!($function), $name, $description)
    }},
    ($function:expr, $description:expr) => {{
        let name = super::name_from_rust_path(stringify!($function));
        FunctionCommandBuilder::new(wrap!($function), name, $description)
    }},
}

/// Macro for making a base command containing a function-command mapping
/// with the provided name, description and a function.
///
/// Example:
/// ```
/// command!(path::to::command, "description")
/// // Equivalent to:
/// // command!(path::to::command, "command", "description")
/// ```
/// Expands to
/// ```
/// MappedCommand::new(func!(path::to::command, "command", "description"))
/// ```
pub macro command {
    ($function:expr, $name:expr, $description:expr) => {{
        MappedCommand::new(func!($function, $name, $description))
    }},
    ($function:expr, $description:expr) => {{
        let name = super::name_from_rust_path(stringify!($function));
        MappedCommand::new(func!($function, name, $description))
    }},
}

/// Creates a subcommand group with the provided name and description.
pub fn subgroup(name: &'static str, description: &'static str) -> FunctionCommandGroupBuilder {
    FunctionCommandGroupBuilder::new(name, description)
}

/// A base command type that contains command metadata and mapped functions for this and its subcommands.
#[derive(Debug, Clone)]
pub struct MappedCommand {
    pub cmd: FunctionCommand,
    pub help: String,
    pub default_member_permissions: Option<Permissions>,
    pub dm_permission: Option<bool>,
}

impl MappedCommand {
    /// Creates a new default base command with a function-command mapping.
    pub fn new(cmd: impl Into<FunctionCommand>) -> Self {
        Self {
            cmd: cmd.into(),
            help: String::new(),
            default_member_permissions: None,
            dm_permission: None,
        }
    }

    /// Set default permissions required for a guild member to run the command.
    ///
    /// Setting this [`Permissions::empty()`] will prohibit anyone from running
    /// the command, except for guild administrators.
    pub fn default_member_permissions(mut self, default_member_permissions: Permissions) -> Self {
        self.default_member_permissions = Some(default_member_permissions);
        self
    }

    /// Set default permissions required for a guild member to run the command, if not yet set.
    ///
    /// Setting this [`Permissions::empty()`] will prohibit anyone from running
    /// the command, except for guild administrators.
    pub fn default_member_permissions_or(
        mut self,
        default_member_permissions: Permissions,
    ) -> Self {
        self.default_member_permissions
            .get_or_insert(default_member_permissions);
        self
    }

    /// Set whether the command is available in DMs.
    ///
    /// This is only relevant for globally-scoped commands. By default, commands are visible in DMs.
    pub fn dm_permission(mut self, dm_permission: bool) -> Self {
        self.dm_permission = Some(dm_permission);
        self
    }

    /// Set whether the command is available in DMs, if not yet set.
    ///
    /// This is only relevant for globally-scoped commands. By default, commands are visible in DMs.
    pub fn dm_permission_or(mut self, dm_permission: bool) -> Self {
        self.dm_permission.get_or_insert(dm_permission);
        self
    }

    /// Add a command option to the base command.
    pub fn option(mut self, option: impl Into<MappedCommandOption>) -> Self {
        self.cmd.add(option);
        self
    }

    /// Try to convert to a twilight command, this will fail if the validation fails.
    pub fn to_command(&self) -> AnyResult<Command> {
        self.try_into()
    }

    /// Returns `(name, command)` pair where `name` is the base command name,
    /// and `command` is the `MappedCommand` itself.
    pub fn named(self) -> (&'static str, Self) {
        (self.cmd.name, self)
    }
}

// Slight abuse for convenience.
impl std::ops::Deref for MappedCommand {
    type Target = FunctionCommand;

    fn deref(&self) -> &Self::Target {
        &self.cmd
    }
}

impl TryFrom<MappedCommand> for Command {
    type Error = anyhow::Error;

    fn try_from(value: MappedCommand) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}

impl TryFrom<&MappedCommand> for Command {
    type Error = anyhow::Error;

    fn try_from(value: &MappedCommand) -> Result<Self, Self::Error> {
        let cmd = Command {
            application_id: None,
            default_member_permissions: value.default_member_permissions,
            dm_permission: value.dm_permission,
            description: value.cmd.description.to_string(),
            description_localizations: None,
            guild_id: None,
            id: None,
            kind: CommandType::ChatInput,
            name: value.cmd.name.to_string(),
            name_localizations: None,
            options: value.cmd.options.iter().cloned().map(Into::into).collect(),
            version: Id::new(1),
        };

        // Check for validity.
        validate_command(&cmd)?;

        Ok(cmd)
    }
}

#[derive(Debug, Clone)]
pub enum MappedCommandOption {
    Arg(CommandOption),
    Sub(FunctionCommand),
    Group(FunctionCommandGroup),
}

/// Shortcut for creating `From` impls for `MappedCommandOption`.
macro impl_arg_builder($b:ty) {
    impl From<$b> for MappedCommandOption {
        fn from(opt: $b) -> Self {
            Self::Arg(opt.build())
        }
    }
}

impl_arg_builder!(AttachmentBuilder);
impl_arg_builder!(BooleanBuilder);
impl_arg_builder!(MentionableBuilder);
impl_arg_builder!(RoleBuilder);
impl_arg_builder!(UserBuilder);
impl_arg_builder!(ChannelBuilder);
impl_arg_builder!(IntegerBuilder);
impl_arg_builder!(NumberBuilder);
impl_arg_builder!(StringBuilder);

impl From<FunctionCommand> for MappedCommandOption {
    fn from(sub: FunctionCommand) -> Self {
        Self::Sub(sub)
    }
}

impl From<FunctionCommandBuilder> for MappedCommandOption {
    fn from(sub: FunctionCommandBuilder) -> Self {
        sub.build().into()
    }
}

impl From<FunctionCommandGroup> for MappedCommandOption {
    fn from(sub: FunctionCommandGroup) -> Self {
        Self::Group(sub)
    }
}

impl From<FunctionCommandGroupBuilder> for MappedCommandOption {
    fn from(sub: FunctionCommandGroupBuilder) -> Self {
        sub.build().into()
    }
}

impl TryFrom<CommandOption> for MappedCommandOption {
    type Error = anyhow::Error;

    fn try_from(value: CommandOption) -> Result<Self, Self::Error> {
        match value {
            CommandOption::SubCommand(_) | CommandOption::SubCommandGroup(_) => Err(
                anyhow::anyhow!("Cannot create subcommands without attached function"),
            ),
            opt => Ok(Self::Arg(opt)),
        }
    }
}

impl From<MappedCommandOption> for CommandOption {
    fn from(opt: MappedCommandOption) -> Self {
        match opt {
            MappedCommandOption::Arg(opt) => opt,
            MappedCommandOption::Sub(opt) => CommandOption::SubCommand(OptionsCommandOptionData {
                description: opt.description.to_string(),
                description_localizations: None,
                name: opt.name.to_string(),
                name_localizations: None,
                options: opt.options.into_iter().map(Into::into).collect(),
            }),
            MappedCommandOption::Group(opt) => {
                CommandOption::SubCommandGroup(OptionsCommandOptionData {
                    description: opt.description.to_string(),
                    description_localizations: None,
                    name: opt.name.to_string(),
                    name_localizations: None,
                    options: opt
                        .subs
                        .into_iter()
                        .map(Into::<MappedCommandOption>::into)
                        .map(Into::into)
                        .collect(),
                })
            },
        }
    }
}

/// A command with a mapped function that executes when this command is ran.
#[derive(Clone)]
pub struct FunctionCommand {
    pub name: &'static str,
    pub description: &'static str,
    pub function: CommandFn,
    pub options: Vec<MappedCommandOption>,
}

impl std::fmt::Debug for FunctionCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionCommand")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("function", &"CommandFn")
            .field("options", &self.options)
            .finish()
    }
}

impl FunctionCommand {
    /// Add an option to the function-command mapping.
    fn add(&mut self, option: impl Into<MappedCommandOption>) {
        self.options.push(option.into());
    }
}

impl From<FunctionCommandBuilder> for FunctionCommand {
    fn from(builder: FunctionCommandBuilder) -> Self {
        builder.build()
    }
}

/// Builder for function-command mapping.
#[derive(Debug, Clone)]
pub struct FunctionCommandBuilder(FunctionCommand);

impl FunctionCommandBuilder {
    /// Creates a new command builder with the provided function, name and description.
    pub fn new(function: CommandFn, name: &'static str, description: &'static str) -> Self {
        Self(FunctionCommand {
            name,
            description,
            function,
            options: Vec::new(),
        })
    }

    /// Add an option to the command.
    pub fn option(mut self, option: impl Into<MappedCommandOption>) -> Self {
        self.0.options.push(option.into());
        self
    }

    /// Finalize the command.
    pub fn build(self) -> FunctionCommand {
        self.0
    }
}

/// A command group.
#[derive(Debug, Clone)]
pub struct FunctionCommandGroup {
    name: &'static str,
    description: &'static str,
    subs: Vec<FunctionCommand>,
}

impl From<FunctionCommandGroupBuilder> for FunctionCommandGroup {
    fn from(builder: FunctionCommandGroupBuilder) -> Self {
        builder.build()
    }
}

/// Builder for subcommand group.
#[derive(Debug, Clone)]
pub struct FunctionCommandGroupBuilder(FunctionCommandGroup);

impl FunctionCommandGroupBuilder {
    /// Create a new subcommand group builder.
    pub fn new(name: &'static str, description: &'static str) -> Self {
        Self(FunctionCommandGroup {
            name,
            description,
            subs: Vec::new(),
        })
    }

    /// Add a subcommand to the group.
    pub fn sub(mut self, option: impl Into<FunctionCommand>) -> Self {
        self.0.subs.push(option.into());
        self
    }

    /// Finalize the group.
    pub fn build(self) -> FunctionCommandGroup {
        self.0
    }
}
