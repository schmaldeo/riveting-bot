pub mod alias;
pub mod bot;
pub mod config;
pub mod perms;
pub mod roles;
pub mod scheduler;
pub mod silence;

#[cfg(feature = "bulk-delete")]
pub mod bulk {
    use twilight_model::id::marker::{ChannelMarker, MessageMarker};
    use twilight_model::id::Id;

    use crate::commands_v2::prelude::*;
    use crate::utils::ExecModelExt;
    // use crate::utils::prelude::*;

    const MAX_DELETE: i64 = 100;

    /// Command: Delete a bunch of messages at once.
    #[derive(Default)]
    pub struct BulkDelete {
        timestamp: i64,
        amount: Option<i64>,
        channel_id: Option<Id<ChannelMarker>>,
        message_id: Option<Id<MessageMarker>>,
    }

    impl Command for BulkDelete {
        type Data = Self;

        async fn uber(ctx: Context, data: Self::Data) -> CommandResult {
            const TWO_WEEKS_SECS: i64 = 60 * 60 * 24 * 7 * 2;

            let two_weeks_ago = data.timestamp - TWO_WEEKS_SECS;

            let Some(count) = data.amount else {
                return Err(CommandError::MissingArgs);
            };

            let Ok(delete_count) = count.max(MAX_DELETE).try_into() else {
                return Err(CommandError::UnexpectedArgs(format!("Could not parse delete count: '{count}'")))
            };

            println!("{delete_count:?}");

            if delete_count == 0 {
                return Ok(Response::Clear);
            }

            let Some(channel_id) = data.channel_id else {
                return Err(CommandError::MissingArgs);
            };

            let Some(message_id) = data.message_id else {
                return Err(CommandError::MissingArgs); // FIXME
            };

            // Fetch and filter messages that are not older than two weeks.
            let msgs: Vec<_> = ctx
                .http
                .channel_messages(channel_id)
                .around(message_id)
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
                ctx.http.delete_messages(channel_id, &msgs).exec().await?;
            } else if let Some(msg) = msgs.first() {
                ctx.http.delete_message(channel_id, *msg).exec().await?;
            }

            Ok(Response::Clear)
        }

        async fn classic(ctx: Context, req: ClassicRequest) -> CommandResult {
            println!("{:?}", req.args);
            Self::uber(ctx, Self {
                timestamp: req.message.timestamp.as_secs(),
                amount: req.args.first().and_then(|a| a.value.clone().integer()),
                channel_id: Some(req.message.channel_id),
                message_id: Some(req.message.id),
            })
            .await
        }

        async fn slash(ctx: Context, req: SlashRequest) -> CommandResult {
            println!("{:?}", req.args);
            Self::uber(ctx, Self {
                timestamp: chrono::Utc::now().timestamp(),
                amount: req.args.first().and_then(|a| a.value.clone().integer()),
                channel_id: req.interaction.channel_id,
                message_id: None,
            })
            .await
        }
    }
}
