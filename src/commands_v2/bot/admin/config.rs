use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;

/// Command: Manage guild configuration.
pub struct Config;

impl Command for Config {
    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        Ok(Response::None)
    }
}

/// Command: Get a guild configuration value.
pub struct Get;

impl Command for Get {
    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        Ok(Response::None)
    }
}

/// Command: Set a guild configuration value.
pub struct Set;

impl Command for Set {
    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        Ok(Response::None)
    }
}

/// Command: Clean config from dangling id references and other expired things.
pub struct Cleanup;

impl Command for Cleanup {
    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        Ok(Response::None)
    }
}
