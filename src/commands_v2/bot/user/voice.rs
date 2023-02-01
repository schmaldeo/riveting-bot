use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;

/// Command: Voice channel controls.
#[derive(Default)]
pub struct Voice;

impl Voice {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Tell the bot to connect to a voice channel.
#[derive(Default)]
pub struct Join;

impl Join {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Tell the bot to disconnect from a voice channel.
#[derive(Default)]
pub struct Leave;

impl Leave {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}
