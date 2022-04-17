use std::str::SplitWhitespace;

use twilight_model::application::component::button::ButtonStyle;
use twilight_model::application::component::{ActionRow, Button, Component};
use twilight_model::channel::message::MessageFlags;
use twilight_model::channel::{Message, ReactionType};

use crate::commands::CommandFunction;
use crate::utils::*;
use crate::Context;

/// Command: Setup a reaction-roles message.
#[derive(Debug, Default)]
pub struct Roles;

#[async_trait]
impl CommandFunction for Roles {
    async fn execute(
        &self,
        ctx: &Context,
        msg: &Message,
        _args: SplitWhitespace<'_>,
    ) -> AnyResult<()> {
        let a = Component::Button(Button {
            custom_id: Some("aaa".into()),
            disabled: false,
            emoji: None,
            label: Some("abc".into()),
            style: ButtonStyle::Primary,
            url: None,
        });

        let b = Component::Button(Button {
            custom_id: Some("bbb".into()),
            disabled: false,
            emoji: Some(ReactionType::Unicode {
                name: "🚗".into()
            }),
            label: Some("cba".into()),
            style: ButtonStyle::Success,
            url: None,
        });

        let components = vec![Component::ActionRow(ActionRow {
            components: [a, b].into(),
        })];

        let res = ctx
            .http
            .create_message(msg.channel_id)
            .flags(MessageFlags::EPHEMERAL)
            .reply(msg.id)
            .content("content")?
            .components(&components)?
            .exec()
            .await;

        let model = match res {
            Ok(k) => k.model().await.unwrap(),
            Err(e) => {
                match e.kind() {
                    twilight_http::error::ErrorType::BuildingRequest => todo!(),
                    twilight_http::error::ErrorType::ChunkingResponse => todo!(),
                    twilight_http::error::ErrorType::CreatingHeader { name } => todo!(),
                    twilight_http::error::ErrorType::Json => todo!(),
                    twilight_http::error::ErrorType::Parsing { body } => todo!(),
                    twilight_http::error::ErrorType::RatelimiterTicket => todo!(),
                    twilight_http::error::ErrorType::RequestCanceled => todo!(),
                    twilight_http::error::ErrorType::RequestError => todo!(),
                    twilight_http::error::ErrorType::RequestTimedOut => todo!(),
                    twilight_http::error::ErrorType::Response {
                        body,
                        error,
                        status,
                    } => {
                        let msg = String::from_utf8_lossy(body);
                        eprintln!("{}", msg);
                    }
                    twilight_http::error::ErrorType::ServiceUnavailable { response } => todo!(),
                    twilight_http::error::ErrorType::Unauthorized => todo!(),
                    _ => todo!(),
                }
                panic!();
            }
        };

        Ok(())
    }
}

/// Command: Delete a bunch of messages at once.
#[derive(Debug, Default)]
pub struct DeleteMessages;

#[async_trait]
impl CommandFunction for DeleteMessages {
    async fn execute(
        &self,
        ctx: &Context,
        msg: &Message,
        _args: SplitWhitespace<'_>,
    ) -> AnyResult<()> {
        const TWO_WEEKS_SECS: i64 = 60 * 60 * 24 * 7 * 2;

        let msgs: Vec<_> = ctx
            .http
            .channel_messages(msg.channel_id)
            .around(msg.id)
            .limit(100)
            .unwrap()
            .send()
            .await?
            .into_iter()
            .filter(|m| msg.timestamp.as_secs() - m.timestamp.as_secs() < TWO_WEEKS_SECS)
            .map(|m| m.id)
            .collect();

        ctx.http
            .delete_messages(msg.channel_id, &msgs)
            .exec()
            .await?;

        Ok(())
    }
}
