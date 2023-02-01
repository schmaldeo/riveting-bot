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
    use crate::utils::prelude::*;

    const MAX_DELETE: i64 = 100;

    /// Command: Delete a bunch of messages at once.
    #[derive(Default)]
    pub struct BulkDelete {
        args: Args,
        timestamp: i64,
        channel_id: Option<Id<ChannelMarker>>,
        message_id: Option<Id<MessageMarker>>,
    }

    impl BulkDelete {
        pub async fn uber(self, ctx: Context) -> CommandResult {
            const TWO_WEEKS_SECS: i64 = 60 * 60 * 24 * 7 * 2;

            let two_weeks_ago = self.timestamp - TWO_WEEKS_SECS;

            let Some(count) = self.args.get("amount").integer() else {
                return Err(CommandError::MissingArgs);
            };

            let Ok(delete_count) = count.min(MAX_DELETE).try_into() else {
                return Err(CommandError::UnexpectedArgs(format!("Could not parse delete count: '{count}'")))
            };

            if delete_count == 0 {
                return Ok(Response::Clear);
            }

            let Some(channel_id) = self.channel_id else {
                return Err(CommandError::MissingArgs);
            };

            let message_id = match self.message_id {
                Some(id) => id,
                None => {
                    match ctx
                        .http
                        .channel_messages(channel_id)
                        .limit(1)?
                        .send()
                        .await?
                        .pop()
                    {
                        Some(m) => m.id,
                        None => return Err(CommandError::MissingArgs),
                    }
                },
            };

            // Fetch and filter messages that are not older than two weeks.
            let msgs: Vec<_> = ctx
                .http
                .channel_messages(channel_id)
                .before(message_id)
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
                ctx.http.delete_messages(channel_id, &msgs).await?;
            } else if let Some(msg) = msgs.first() {
                ctx.http.delete_message(channel_id, *msg).await?;
            }

            Ok(Response::Clear)
        }

        pub async fn classic(ctx: Context, req: ClassicRequest) -> CommandResult {
            Self {
                args: req.args,
                timestamp: req.message.timestamp.as_secs(),
                channel_id: Some(req.message.channel_id),
                message_id: Some(req.message.id),
            }
            .uber(ctx)
            .await
        }

        pub async fn slash(ctx: Context, req: SlashRequest) -> CommandResult {
            Self {
                args: req.args,
                timestamp: chrono::Utc::now().timestamp(),
                channel_id: req.interaction.channel_id,
                message_id: None,
            }
            .uber(ctx)
            .await
        }
    }
}
