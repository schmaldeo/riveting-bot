use std::sync::Arc;

use derive_more::{From, IsVariant, Unwrap};
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::channel::{Attachment, Channel, Message};
use twilight_model::id::marker::{
    AttachmentMarker, ChannelMarker, GenericMarker, MessageMarker, RoleMarker, UserMarker,
};
use twilight_model::id::Id;
use twilight_model::user::User;

// use crate::utils::prelude::*;

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

/// Result value for argument.
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
    Role(Id<RoleMarker>),
    Mention(Id<GenericMarker>),
}

macro_rules! impl_variant_option {
    ( $( $v:vis fn $func:ident ( &self: $var:ident ( $tok:tt ) ) -> $ret:ty { $out:expr } )* ) => {
        $(
            /// Returns `Some` if `self` matches variant, else `None`.
            $v fn $func(&self) -> Option<$ret> {
                match self {
                    Self::$var($tok) => Some($out),
                    _ => None,
                }
            }
        )*
    };
}

impl ArgValue {
    impl_variant_option!(
        pub fn bool(&self: Bool(val)) -> bool { *val }
        pub fn number(&self: Number(val)) -> f64 { *val }
        pub fn integer(&self: Integer(val)) -> i64 { *val }
        pub fn string(&self: String(val)) -> &String { val }
        pub fn channel(&self: Channel(val)) -> &Ref<ChannelMarker, Channel> { val }
        pub fn message(&self: Message(val)) -> &Ref<MessageMarker, Message> { val }
        pub fn attachment(&self: Attachment(val)) -> &Ref<AttachmentMarker, Attachment> { val }
        pub fn user(&self: User(val)) -> &Ref<UserMarker, User> { val }
        pub fn role(&self: Role(val)) -> Id<RoleMarker> { *val }
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
            CommandOptionValue::Role(id) => Ok(Self::Role(id)),
            CommandOptionValue::Focused(_s, _c) => todo!(), // FIXME: To be implemented
            CommandOptionValue::SubCommand(_) | CommandOptionValue::SubCommandGroup(_) => {
                Err("Cannot convert subcommand or group to argument value")
            },
        }
    }
}
