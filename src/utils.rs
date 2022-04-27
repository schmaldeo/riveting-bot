#![allow(unused)]

pub(crate) use anyhow::Result as AnyResult;
pub(crate) use async_trait::async_trait;
pub(crate) use tracing::{debug, error, info, trace, warn};
use twilight_http::request::channel::message::{CreateMessage, GetChannelMessagesConfigured};
use twilight_http::request::channel::GetChannel;
use twilight_http::request::guild::member::GetMember;
use twilight_http::request::guild::role::GetGuildRoles;
use twilight_http::request::guild::GetGuild;
use twilight_http::request::user::GetCurrentUser;
use twilight_http::request::GetUserApplicationInfo;
use twilight_model::channel::{Channel, Message};
use twilight_model::guild::{Guild, Member, Role};
use twilight_model::oauth::Application;
use twilight_model::user::CurrentUser;

/// Universal constants.
pub mod consts {
    pub const EVERYONE: &str = "@everyone";
}

/// A trait to simplify `.exec().await?.model.await` chain.
#[async_trait]
pub trait ExecModelExt {
    type Value;

    /// Send the command by calling `exec()` and `model()`.
    async fn send(self) -> AnyResult<Self::Value>;
}

/// Macro to implement `ExecModelExt` in a one-liner.
macro impl_exec_model_ext($req:ty, $val:ty) {
    #[async_trait]
    impl ExecModelExt for $req {
        type Value = $val;

        async fn send(self) -> AnyResult<Self::Value> {
            self.exec().await?.model().await.map_err(Into::into)
        }
    }
}

impl_exec_model_ext!(CreateMessage<'_>, Message);
impl_exec_model_ext!(GetChannel<'_>, Channel);
impl_exec_model_ext!(GetChannelMessagesConfigured<'_>, Vec<Message>);
impl_exec_model_ext!(GetCurrentUser<'_>, CurrentUser);
impl_exec_model_ext!(GetGuild<'_>, Guild);
impl_exec_model_ext!(GetGuildRoles<'_>, Vec<Role>);
impl_exec_model_ext!(GetMember<'_>, Member);
impl_exec_model_ext!(GetUserApplicationInfo<'_>, Application);

/// Macro to simplify manual non-exhaustive `Debug` impl.
pub macro impl_debug_struct_fields($t:ty, $($field:ident),*) {
    impl std::fmt::Debug for $t {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct(stringify!($t))
            $(.field(stringify!($field), &self.$field))*
            .finish_non_exhaustive()
        }
    }
}
