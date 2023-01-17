use std::borrow::Borrow;
use std::sync::Arc;

use derive_more::{AsMut, AsRef, From, Index, IndexMut, IntoIterator, IsVariant, Unwrap};
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::channel::{Attachment, Channel, Message};
use twilight_model::guild::Role;
use twilight_model::id::marker::{
    AttachmentMarker, ChannelMarker, GenericMarker, MessageMarker, RoleMarker, UserMarker,
};
use twilight_model::id::Id;
use twilight_model::user::User;

use crate::commands_v2::builder::{ArgDesc, ArgKind};
use crate::utils::prelude::*;

/// Contained value that is either type `Ref::Id(Id<A>)` or `Ref::Obj(Box<B>)`.
#[derive(Debug, From, Unwrap, IsVariant)]
pub enum Ref<A, B> {
    Id(Id<A>),
    Obj(Arc<B>),
}

impl<A, B> Clone for Ref<A, B> {
    fn clone(&self) -> Self {
        match self {
            Self::Id(arg0) => Self::Id(*arg0),
            Self::Obj(arg0) => Self::Obj(Arc::clone(arg0)),
        }
    }
}

impl<A, B> Ref<A, B> {
    /// Box an object and return the object variant.
    pub fn from_obj(obj: B) -> Self {
        Self::Obj(Arc::new(obj))
    }
}

/// Wrapper around `Vec<Arg>` for extra features.
#[derive(Debug, Default, Clone, AsMut, AsRef, From, Index, IndexMut, IntoIterator)]
pub struct Args(Vec<Arg>);

impl Args {
    /// Finds argument value by argument name.
    pub fn get(&self, name: &str) -> Option<&ArgValue> {
        self.as_ref()
            .iter()
            .find(|a| a.name == name)
            .map(|a| &a.value)
    }

    pub fn inner(self) -> Vec<Arg> {
        self.0
    }
}

/// A type representing an argument with name and value.
#[derive(Debug, Clone)]
pub struct Arg {
    pub name: String,
    pub value: ArgValue,
}

impl Arg {
    pub fn from_desc(desc: &ArgDesc, text: &str) -> AnyResult<Self> {
        // TODO: Ensure data parameters.
        let value = match desc.kind {
            ArgKind::Bool => ArgValue::Bool(
                text.to_lowercase()
                    .parse()
                    .context("Bool arg parse error")?,
            ),
            ArgKind::Number(_) => ArgValue::Number(text.parse().context("Number arg parse error")?),
            ArgKind::Integer(_) => {
                ArgValue::Integer(text.parse().context("Integer arg parse error")?)
            },
            ArgKind::String(_) => ArgValue::String(text.to_string()),
            ArgKind::Channel(_) => ArgValue::Channel(Ref::Id(
                text.parse().context("Channel arg parse error")?, // FIXME: This might not be a bare id.
            )),
            ArgKind::Message => {
                ArgValue::Message(Ref::Id(text.parse().context("Message arg parse error")?))
            },
            ArgKind::Attachment => {
                ArgValue::Attachment(Ref::Id(text.parse().context("Attachment arg parse error")?))
            },
            ArgKind::User => ArgValue::User(Ref::Id(
                text.parse().context("User arg parse error")?, // FIXME: This might not be a bare id.
            )),
            ArgKind::Role => ArgValue::Role(Ref::Id(
                text.parse().context("Role arg parse error")?, // FIXME: This might not be a bare id.
            )),
            ArgKind::Mention => ArgValue::Mention(
                text.parse().context("Mention arg parse error")?, // FIXME: This might not be a bare id.
            ),
        };

        Ok(Self {
            name: desc.name.to_string(),
            value,
        })
    }
}

/// Argument value type with data.
#[derive(Debug, Clone, Unwrap, IsVariant)]
pub enum ArgValue {
    Bool(bool),
    Number(f64),
    Integer(i64),
    String(String),
    Channel(Ref<ChannelMarker, Channel>),
    Message(Ref<MessageMarker, Message>),
    Attachment(Ref<AttachmentMarker, Attachment>),
    User(Ref<UserMarker, User>),
    Role(Ref<RoleMarker, Role>),
    Mention(Id<GenericMarker>),
}

impl ArgValue {
    impl_variant_option!(
        pub fn bool(&self: Bool(val)) -> bool { *val }
        pub fn number(&self: Number(val)) -> f64 { *val }
        pub fn integer(&self: Integer(val)) -> i64 { *val }
        pub fn string(&self: String(val)) -> &String;
        pub fn channel(&self: Channel(val)) -> &Ref<ChannelMarker, Channel>;
        pub fn message(&self: Message(val)) -> &Ref<MessageMarker, Message>;
        pub fn attachment(&self: Attachment(val)) -> &Ref<AttachmentMarker, Attachment>;
        pub fn user(&self: User(val)) -> &Ref<UserMarker, User>;
        pub fn role(&self: Role(val)) -> &Ref<RoleMarker, Role>;
        pub fn mention(&self: Mention(val)) -> Id<GenericMarker> { *val }
    );
}

impl TryFrom<CommandOptionValue> for ArgValue {
    type Error = &'static str;

    fn try_from(value: CommandOptionValue) -> Result<Self, Self::Error> {
        match value {
            CommandOptionValue::Boolean(b) => Ok(Self::Bool(b)),
            CommandOptionValue::Number(n) => Ok(Self::Number(n)),
            CommandOptionValue::Integer(i) => Ok(Self::Integer(i)),
            CommandOptionValue::String(s) => Ok(Self::String(s)),
            CommandOptionValue::Channel(id) => Ok(Self::Channel(Ref::Id(id))),
            CommandOptionValue::Mentionable(id) => Ok(Self::Mention(id)),
            CommandOptionValue::Attachment(id) => Ok(Self::Attachment(Ref::Id(id))),
            CommandOptionValue::User(id) => Ok(Self::User(Ref::Id(id))),
            CommandOptionValue::Role(id) => Ok(Self::Role(Ref::Id(id))),
            CommandOptionValue::Focused(_s, _c) => todo!(), // FIXME: To be implemented
            CommandOptionValue::SubCommand(_) | CommandOptionValue::SubCommandGroup(_) => {
                Err("Cannot convert subcommand or group to argument value")
            },
        }
    }
}

/// Extension for `Option<ArgValue>` to get an option of matching variant.
pub trait ArgValueExt {
    fn bool(&self) -> Option<bool>;
    fn number(&self) -> Option<f64>;
    fn integer(&self) -> Option<i64>;
    fn string(&self) -> Option<&String>;
    fn channel(&self) -> Option<&Ref<ChannelMarker, Channel>>;
    fn message(&self) -> Option<&Ref<MessageMarker, Message>>;
    fn attachment(&self) -> Option<&Ref<AttachmentMarker, Attachment>>;
    fn user(&self) -> Option<&Ref<UserMarker, User>>;
    fn role(&self) -> Option<&Ref<RoleMarker, Role>>;
    fn mention(&self) -> Option<Id<GenericMarker>>;
}

impl<T> ArgValueExt for Option<T>
where
    T: Borrow<ArgValue>,
{
    fn bool(&self) -> Option<bool> {
        self.as_ref().and_then(|v| v.borrow().bool())
    }

    fn number(&self) -> Option<f64> {
        self.as_ref().and_then(|v| v.borrow().number())
    }

    fn integer(&self) -> Option<i64> {
        self.as_ref().and_then(|v| v.borrow().integer())
    }

    fn string(&self) -> Option<&String> {
        self.as_ref().and_then(|v| v.borrow().string())
    }

    fn channel(&self) -> Option<&Ref<ChannelMarker, Channel>> {
        self.as_ref().and_then(|v| v.borrow().channel())
    }

    fn message(&self) -> Option<&Ref<MessageMarker, Message>> {
        self.as_ref().and_then(|v| v.borrow().message())
    }

    fn attachment(&self) -> Option<&Ref<AttachmentMarker, Attachment>> {
        self.as_ref().and_then(|v| v.borrow().attachment())
    }

    fn user(&self) -> Option<&Ref<UserMarker, User>> {
        self.as_ref().and_then(|v| v.borrow().user())
    }

    fn role(&self) -> Option<&Ref<RoleMarker, Role>> {
        self.as_ref().and_then(|v| v.borrow().role())
    }

    fn mention(&self) -> Option<Id<GenericMarker>> {
        self.as_ref().and_then(|v| v.borrow().mention())
    }
}
