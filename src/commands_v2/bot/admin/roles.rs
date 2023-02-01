use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;

#[derive(Default)]
pub struct Roles;

impl Roles {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}
