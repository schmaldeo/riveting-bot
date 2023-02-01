use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;

/// Command: Create or edit bot messages.
#[derive(Default)]
pub struct Bot;

impl Bot {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Post a message as the bot.
#[derive(Default)]
pub struct Say;

impl Say {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Edit a message created by the bot (can be anything).
#[derive(Default)]
pub struct Edit;

impl Edit {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}
