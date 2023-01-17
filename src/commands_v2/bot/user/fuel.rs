use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;

/// Command: Calculate fuel required.
#[derive(Default)]
pub struct Fuel {
    args: Args,
}

impl Command for Fuel {
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

    async fn user(ctx: Context, _req: UserRequest) -> CommandResult {
        Self::uber(ctx, Default::default()).await
    }

    async fn message(ctx: Context, _req: MessageRequest) -> CommandResult {
        Self::uber(ctx, Default::default()).await
    }
}
