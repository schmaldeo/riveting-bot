use std::sync::Arc;

use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::Message;

use crate::commands_v2::arg::ArgValue;
use crate::commands_v2::builder::BaseCommand;

#[derive(Debug, Clone)]
pub struct Arg {
    pub name: String,
    pub value: ArgValue,
}

/// Classic command request with preprocessed arguments and original message.
#[derive(Debug, Clone)]
pub struct ClassicRequest {
    pub args: Vec<Arg>,
    pub message: Arc<Message>,
    pub command: Arc<BaseCommand>,
}

impl ClassicRequest {
    pub fn new(command: Arc<BaseCommand>, message: Arc<Message>) -> Self {
        Self {
            args: Vec::new(),
            message,
            command,
        }
    }
}

/// Slash command request with preprocessed arguments and interaction data.
#[derive(Debug, Clone)]
pub struct SlashRequest {
    pub command: Arc<BaseCommand>,
    pub interaction: Arc<Interaction>,
    pub data: Arc<CommandData>,
    pub args: Vec<Arg>,
}
impl SlashRequest {
    pub fn new(
        command: Arc<BaseCommand>,
        interaction: Arc<Interaction>,
        data: Arc<CommandData>,
        args: Vec<Arg>,
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
}

impl MessageRequest {
    pub fn new(
        command: Arc<BaseCommand>,
        interaction: Arc<Interaction>,
        data: Arc<CommandData>,
    ) -> Self {
        Self {
            command,
            interaction,
            data,
        }
    }
}

/// User command request with command and interaction data.
#[derive(Debug, Clone)]
pub struct UserRequest {
    pub command: Arc<BaseCommand>,
    pub interaction: Arc<Interaction>,
    pub data: Arc<CommandData>,
}

impl UserRequest {
    pub fn new(
        command: Arc<BaseCommand>,
        interaction: Arc<Interaction>,
        data: Arc<CommandData>,
    ) -> Self {
        Self {
            command,
            interaction,
            data,
        }
    }
}
