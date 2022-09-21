use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;

/// Command: Create or edit bot messages.
#[derive(Default)]
pub struct Bot;

impl Command for Bot {
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

/// Command: Post a message as the bot.
#[derive(Default)]
pub struct Say;

impl Command for Say {
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

/// Command: Edit a message created by the bot (can be anything).
#[derive(Default)]
pub struct Edit;

impl Command for Edit {
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
