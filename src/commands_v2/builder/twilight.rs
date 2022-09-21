use thiserror::Error;
use twilight_model::application::command::{
    ChannelCommandOptionData, ChoiceCommandOptionData, Command, CommandOption, CommandOptionChoice,
    CommandOptionValue, CommandType, Number, NumberCommandOptionData, OptionsCommandOptionData,
};
use twilight_util::builder::command::{
    AttachmentBuilder, BooleanBuilder, ChannelBuilder, CommandBuilder, IntegerBuilder,
    MentionableBuilder, NumberBuilder, RoleBuilder, StringBuilder, UserBuilder,
};

pub use self::traits::*;
use crate::commands_v2::builder::{BaseCommand, ChannelData, NumericalData, StringData};
use crate::utils::prelude::*;

pub mod traits {
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

    pub trait CommandOptionExt {
        fn name(&self) -> &str;
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

impl CommandOptionExt for CommandOption {
    fn name(&self) -> &str {
        match self {
            Self::SubCommand(data) => &data.name,
            Self::SubCommandGroup(data) => &data.name,
            Self::String(data) => &data.name,
            Self::Integer(data) => &data.name,
            Self::Boolean(data) => &data.name,
            Self::User(data) => &data.name,
            Self::Channel(data) => &data.name,
            Self::Role(data) => &data.name,
            Self::Mentionable(data) => &data.name,
            Self::Number(data) => &data.name,
            Self::Attachment(data) => &data.name,
        }
    }
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

/// Validates options in the command.
pub fn validate_command(cmd: &Command) -> Result<(), CommandValidationError> {
    use twilight_validate::command::option_name as validate_option_name;

    /// Checks the validity of each option and for local multiples of same option names.
    fn validate_options(options: &[CommandOption]) -> Result<(), CommandValidationError> {
        options.iter().enumerate().try_for_each(|(idx, opt)| {
            let name = opt.name();

            // Check for local ambiguity:
            // All the rest of the options must not have this name.
            if let Some(slice) = options.get(idx + 1..) {
                if slice.iter().any(|c| c.name() == name) {
                    return Err(CommandValidationError::AmbiguousName(format!(
                        "Duplicate name '{name}' in option of kind '{}'",
                        opt.kind().kind()
                    )));
                }
            }

            // Check for valid name and any subcommands or groups.
            match opt {
                CommandOption::SubCommand(data) | CommandOption::SubCommandGroup(data) => {
                    validate_option_name(name)?;
                    validate_options(&data.options)
                },
                _ => Ok(validate_option_name(name)?),
            }
        })
    }

    if matches!(cmd.kind, CommandType::User | CommandType::Message) {
        // Check with twilight's validations.
        twilight_validate::command::name(&cmd.name)?;

        // Other manual checks.
        if !cmd.options.is_empty() {
            return Err(CommandValidationError::GuiOptions);
        }
    } else {
        // Check with twilight's validations.
        validate_option_name(&cmd.name)?;
        twilight_validate::command::command(cmd)?; // Does not check for options.
        twilight_validate::command::options(&cmd.options)?; // Does not check subcommand/group names or multiple uses.

        // Other manual checks.
        validate_options(&cmd.options)?;
    }

    Ok(())
}

#[derive(Debug, Error)]
pub enum CommandValidationError {
    /// Multiple uses of same option name.
    #[error("Option names must be locally unique: {0}")]
    AmbiguousName(String),

    /// Options in GUI-based commands.
    #[error("GUI-based commands cannot have options")]
    GuiOptions,

    /// Twilight's validation error.
    #[error(transparent)]
    Twilight(#[from] twilight_validate::command::CommandValidationError),

    /// Other errors that are or can be converted to `anyhow::Error`.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub struct SlashCommand(Command);

impl TryFrom<&BaseCommand> for SlashCommand {
    type Error = CommandValidationError;

    fn try_from(value: &BaseCommand) -> Result<Self, Self::Error> {
        let mut cmd = CommandBuilder::new(
            value.command.name,
            value.command.description,
            CommandType::ChatInput,
        )
        .dm_permission(value.dm_enabled)
        .default_member_permissions(value.member_permissions)
        .build();

        cmd.options = value
            .command
            .options
            .iter()
            .map(|o| o.try_into().map_err(anyhow::Error::from))
            .try_collect()?;

        validate_command(&cmd)?;

        Ok(Self(cmd))
    }
}

impl From<SlashCommand> for Command {
    fn from(value: SlashCommand) -> Self {
        value.0
    }
}

pub struct UserCommand(Command);

impl TryFrom<&BaseCommand> for UserCommand {
    type Error = CommandValidationError;

    fn try_from(value: &BaseCommand) -> Result<Self, Self::Error> {
        let cmd = CommandBuilder::new(value.command.name, "", CommandType::User)
            .dm_permission(value.dm_enabled)
            .default_member_permissions(value.member_permissions)
            .build();

        validate_command(&cmd)?;

        Ok(Self(cmd))
    }
}

impl From<UserCommand> for Command {
    fn from(value: UserCommand) -> Self {
        value.0
    }
}

pub struct MessageCommand(Command);

impl TryFrom<&BaseCommand> for MessageCommand {
    type Error = CommandValidationError;

    fn try_from(value: &BaseCommand) -> Result<Self, Self::Error> {
        let cmd = CommandBuilder::new(value.command.name, "", CommandType::Message)
            .dm_permission(value.dm_enabled)
            .default_member_permissions(value.member_permissions)
            .build();

        validate_command(&cmd)?;

        Ok(Self(cmd))
    }
}

impl From<MessageCommand> for Command {
    fn from(value: MessageCommand) -> Self {
        value.0
    }
}

impl From<&super::CommandOption> for CommandOption {
    fn from(value: &super::CommandOption) -> Self {
        match value {
            super::CommandOption::Arg(arg) => arg.into(),
            super::CommandOption::Sub(sub) => sub.into(),
            super::CommandOption::Group(group) => group.into(),
        }
    }
}

impl From<&super::CommandFunction> for CommandOption {
    fn from(value: &super::CommandFunction) -> Self {
        Self::SubCommand(OptionsCommandOptionData {
            description: value.description.to_string(),
            name: value.name.to_string(),
            options: value.options.iter().map(Into::into).collect(),
            ..Default::default()
        })
    }
}

impl From<&super::CommandGroup> for CommandOption {
    fn from(value: &super::CommandGroup) -> Self {
        Self::SubCommandGroup(OptionsCommandOptionData {
            description: value.description.to_string(),
            name: value.name.to_string(),
            options: value.subs.iter().map(Into::into).collect(),
            ..Default::default()
        })
    }
}

impl From<&super::ArgDesc> for CommandOption {
    fn from(value: &super::ArgDesc) -> Self {
        match &value.kind {
            // Boolean(BaseCommandOptionData),
            super::ArgKind::Bool => boolean(value.name, value.description)
                .required(value.required)
                .build(),

            // Number(NumberCommandOptionData),
            super::ArgKind::Number(NumericalData { min, max, choices }) => {
                Self::Number(NumberCommandOptionData {
                    choices: choices
                        .iter()
                        .map(|(name, val)| CommandOptionChoice::Number {
                            name: name.to_owned(),
                            name_localizations: None,
                            value: Number(*val),
                        })
                        .collect(),
                    description: value.description.to_string(),
                    max_value: max.map(Number).map(CommandOptionValue::Number),
                    min_value: min.map(Number).map(CommandOptionValue::Number),
                    name: value.name.to_string(),
                    required: value.required,
                    ..Default::default()
                })
            },

            // Integer(NumberCommandOptionData),
            super::ArgKind::Integer(NumericalData { min, max, choices }) => {
                Self::Integer(NumberCommandOptionData {
                    choices: choices
                        .iter()
                        .map(|(name, val)| CommandOptionChoice::Int {
                            name: name.to_owned(),
                            name_localizations: None,
                            value: *val,
                        })
                        .collect(),
                    description: value.description.to_string(),
                    max_value: max.map(CommandOptionValue::Integer),
                    min_value: min.map(CommandOptionValue::Integer),
                    name: value.name.to_string(),
                    required: value.required,
                    ..Default::default()
                })
            },

            // String(ChoiceCommandOptionData),
            super::ArgKind::String(StringData {
                min_length,
                max_length,
                choices,
            }) => Self::String(ChoiceCommandOptionData {
                choices: choices
                    .iter()
                    .map(|(name, val)| CommandOptionChoice::String {
                        name: name.to_owned(),
                        name_localizations: None,
                        value: val.to_owned(),
                    })
                    .collect(),
                description: value.description.to_string(),
                max_length: max_length.to_owned(),
                min_length: min_length.to_owned(),
                name: value.name.to_string(),
                required: value.required,
                ..Default::default()
            }),

            // Channel(ChannelCommandOptionData),
            super::ArgKind::Channel(ChannelData { channel_types }) => {
                Self::Channel(ChannelCommandOptionData {
                    channel_types: channel_types.to_owned(),
                    description: value.description.to_string(),
                    description_localizations: None,
                    name: value.name.to_string(),
                    name_localizations: None,
                    required: value.required,
                })
            },

            // NOTE: Instead of message reference, get the message id as string.
            super::ArgKind::Message(_) => Self::String(ChoiceCommandOptionData {
                description: value.description.to_string(),
                max_length: Some(32),
                min_length: Some(1),
                name: value.name.to_string(),
                required: value.required,
                ..Default::default()
            }),

            // User(BaseCommandOptionData),
            super::ArgKind::User => user(value.name, value.description)
                .required(value.required)
                .build(),

            // Attachment(BaseCommandOptionData),
            super::ArgKind::Attachment => attachment(value.name, value.description)
                .required(value.required)
                .build(),

            // Role(BaseCommandOptionData),
            super::ArgKind::Role => role(value.name, value.description)
                .required(value.required)
                .build(),

            // Mentionable(BaseCommandOptionData),
            super::ArgKind::Mention => mention(value.name, value.description)
                .required(value.required)
                .build(),
        }
    }
}
