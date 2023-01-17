use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;

/// Command: Voice channel controls.
#[derive(Default)]
pub struct Voice;

impl Command for Voice {
    type Data = Self;

    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        Ok(Response::Clear)
    }

    async fn classic(ctx: Context, _req: ClassicRequest) -> CommandResult {
        Self::uber(ctx, Default::default()).await
    }

    async fn slash(ctx: Context, _req: SlashRequest) -> CommandResult {
        Self::uber(ctx, Default::default()).await
    }
}

/// Command: Tell the bot to connect to a voice channel.
#[derive(Default)]
pub struct Join;

impl Command for Join {
    type Data = Self;

    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        todo!()
    }
}

/// Command: Tell the bot to disconnect from a voice channel.
#[derive(Default)]
pub struct Leave;

impl Command for Leave {
    type Data = Self;

    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        todo!()
    }
}
