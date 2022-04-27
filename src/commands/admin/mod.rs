use twilight_model::application::component::button::ButtonStyle;
use twilight_model::application::component::{ActionRow, Button, Component};
use twilight_model::channel::message::MessageFlags;
use twilight_model::channel::ReactionType;

use crate::commands::{CommandContext, CommandError, CommandResult};
use crate::utils::*;

pub mod config;

/// Command: Setup a reaction-roles message.
pub async fn roles(cc: CommandContext<'_>) -> CommandResult {
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

/// Command: Delete a bunch of messages at once.
pub async fn delete_messages(cc: CommandContext<'_>) -> CommandResult {
    const TWO_WEEKS_SECS: i64 = 60 * 60 * 24 * 7 * 2;

    let two_weeks_ago = cc.msg.timestamp.as_secs() - TWO_WEEKS_SECS;

    let mut delete_count = 100;

    if !cc.args.is_empty() {
        delete_count = cc.args.parse().or(Err(CommandError::UnexpectedArgs))?;
    }

    // Fetch and filter messages that are not older than two weeks.
    let msgs: Vec<_> = cc
        .http
        .channel_messages(cc.msg.channel_id)
        .around(cc.msg.id)
        .limit(delete_count)?
        .send()
        .await?
        .into_iter()
        .filter(|m| two_weeks_ago < m.timestamp.as_secs())
        .map(|m| m.id)
        .collect();

    // Delete the messages.
    if msgs.len() > 1 {
        // Bulk delete must have 2 to 100 messages.
        cc.http
            .delete_messages(cc.msg.channel_id, &msgs)
            .exec()
            .await?;
    } else {
        // Delete only the sent message itself.
        cc.http
            .delete_message(cc.msg.channel_id, cc.msg.id)
            .exec()
            .await?;
    }

    Ok(())
}
