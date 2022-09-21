use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;

#[derive(Default)]
pub struct Roles;

impl Command for Roles {
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
