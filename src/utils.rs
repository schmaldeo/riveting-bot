#![allow(unused)]

pub(crate) use anyhow::Result as AnyResult;
pub(crate) use async_trait::async_trait;
pub(crate) use tracing::{debug, error, info, trace, warn};
use twilight_http::request::channel::message::{
    CreateMessage, DeleteMessages, GetChannelMessagesConfigured,
};
use twilight_http::request::channel::GetChannel;
use twilight_http::request::guild::member::GetMember;
use twilight_http::request::guild::role::GetGuildRoles;
use twilight_http::request::guild::GetGuild;
use twilight_model::channel::{Channel, Message};
use twilight_model::guild::{Guild, Member, Role};

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
impl_exec_model_ext!(GetGuild<'_>, Guild);
impl_exec_model_ext!(GetGuildRoles<'_>, Vec<Role>);
impl_exec_model_ext!(GetMember<'_>, Member);

// #[async_trait]
// impl ExecModelExt for CreateMessage<'_> {
//     type Value = Message;
//     async fn send(self) -> AnyResult<Self::Value> {
//         self.exec().await?.model().await.map_err(Into::into)
//     }
// }

// #[async_trait]
// impl ExecModelExt for GetMember<'_> {
//     type Value = Member;
//     async fn send(self) -> AnyResult<Self::Value> {
//         self.exec().await?.model().await.map_err(Into::into)
//     }
// }
