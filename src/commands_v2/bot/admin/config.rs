use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;

/// Command: Manage guild configuration.
pub struct Config;

impl Command for Config {
    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        todo!()
    }

    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        Ok(Response::None)
    }
}

/// Command: Get a guild configuration value.
pub struct Get;

impl Command for Get {
    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        todo!()
    }

    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        Ok(Response::None)
    }
}

/// Command: Set a guild configuration value.
pub struct Set;

impl Command for Set {
    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        todo!()
    }

    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        Ok(Response::None)
    }
}

/// Command: Clean config from dangling id references and other expired things.
pub struct Cleanup;

impl Command for Cleanup {
    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        todo!()
    }

    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        Ok(Response::None)
    }
}
