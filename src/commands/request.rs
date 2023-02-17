use std::sync::Arc;

use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::Message;
use twilight_model::id::marker::{MessageMarker, UserMarker};
use twilight_model::id::Id;

use crate::commands::arg::Args;
use crate::commands::builder::BaseCommand;

/// Classic command request with preprocessed arguments and original message.
#[derive(Debug, Clone)]
pub struct ClassicRequest {
    pub command: Arc<BaseCommand>,
    pub message: Arc<Message>,
    pub args: Args,
}

impl ClassicRequest {
    pub fn new(command: Arc<BaseCommand>, message: Arc<Message>, args: Args) -> Self {
        Self {
            command,
            message,
            args,
        }
    }
}

/// Slash command request with preprocessed arguments and interaction data.
#[derive(Debug, Clone)]
pub struct SlashRequest {
    pub command: Arc<BaseCommand>,
    pub interaction: Arc<Interaction>,
    pub data: Arc<CommandData>,
    pub args: Args,
}

impl SlashRequest {
    pub fn new(
        command: Arc<BaseCommand>,
        interaction: Arc<Interaction>,
        data: Arc<CommandData>,
        args: Args,
    ) -> Self {
        Self {
            command,
            interaction,
            data,
            args,
        }
    }
}

/// Message command request with command and interaction data.
#[derive(Debug, Clone)]
pub struct MessageRequest {
    pub command: Arc<BaseCommand>,
    pub interaction: Arc<Interaction>,
    pub data: Arc<CommandData>,
    pub target_id: Id<MessageMarker>,
}

impl MessageRequest {
    pub fn new(
        command: Arc<BaseCommand>,
        interaction: Arc<Interaction>,
        data: Arc<CommandData>,
        target_id: Id<MessageMarker>,
    ) -> Self {
        Self {
            command,
            interaction,
            data,
            target_id,
        }
    }
}

/// User command request with command and interaction data.
#[derive(Debug, Clone)]
pub struct UserRequest {
    pub command: Arc<BaseCommand>,
    pub interaction: Arc<Interaction>,
    pub data: Arc<CommandData>,
    pub target_id: Id<UserMarker>,
}

impl UserRequest {
    pub fn new(
        command: Arc<BaseCommand>,
        interaction: Arc<Interaction>,
        data: Arc<CommandData>,
        target_id: Id<UserMarker>,
    ) -> Self {
        Self {
            command,
            interaction,
            data,
            target_id,
        }
    }
}
