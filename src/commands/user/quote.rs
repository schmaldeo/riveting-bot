use crate::commands::{CommandContext, CommandError, CommandResult};
use crate::utils::prelude::*;

/// Command: Quote a random person or manage quotes.
pub async fn quote(_cc: CommandContext<'_>) -> CommandResult {
    // TODO Get a random quote.
    Err(CommandError::NotImplemented)
}

pub async fn add(cc: CommandContext<'_>) -> CommandResult {
    match &cc.msg.referenced_message {
        Some(rep) => {
            // Create a quote.
            let quote = quote_user(&rep.content, &rep.author.name);
            let _sent = cc
                .http
                .create_message(cc.msg.channel_id)
                .reply(cc.msg.id)
                .content(&quote)?
                .send()
                .await?;
            Ok(())
        },
        None => Err(CommandError::MissingReply),
    }
}

pub async fn remove(cc: CommandContext<'_>) -> CommandResult {
    match &cc.msg.referenced_message {
        Some(_rep) => {
            // Remove a quote.
            let sent = cc
                .http
                .create_message(cc.msg.channel_id)
                .reply(cc.msg.id)
                .content("Quote removed")?
                .send()
                .await?;

            // Delete info message.
            cc.http
                .delete_message(sent.channel_id, sent.id)
                .exec()
                .await?;
            Ok(())
        },
        None => Err(CommandError::MissingReply),
    }
}

fn quote_user(text: &str, user: &str) -> String {
    format!(">>> {text}\n\t*— {user}*")
}
