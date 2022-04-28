use twilight_model::application::component::button::ButtonStyle;
use twilight_model::application::component::{ActionRow, Button, Component};
use twilight_model::channel::message::MessageFlags;
use twilight_model::channel::ReactionType;

use crate::commands::{CommandContext, CommandError, CommandResult};
use crate::utils::*;

/// Command: Setup a reaction-roles message.
pub async fn roles(cc: CommandContext<'_>) -> CommandResult {
    // TODO Just some testing going on here.

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

    let res = cc
        .http
        .create_message(cc.msg.channel_id)
        .flags(MessageFlags::EPHEMERAL)
        .reply(cc.msg.id)
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
                },
                twilight_http::error::ErrorType::ServiceUnavailable { response } => todo!(),
                twilight_http::error::ErrorType::Unauthorized => todo!(),
                _ => todo!(),
            }
            panic!();
        },
    };

    Ok(())
}
