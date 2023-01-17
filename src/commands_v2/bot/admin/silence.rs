use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;

pub struct Mute;

impl Command for Mute {
    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        todo!()
    }
}

pub struct Timeout;

impl Command for Timeout {
    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        todo!()
    }
}
