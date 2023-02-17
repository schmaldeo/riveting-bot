use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;

/// Command: Manage guild configuration.
pub struct Config;

impl Config {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands_v2::builder::*;

        command("config", "Manage guild config.")
            .attach(Self::classic)
            .permissions(Permissions::ADMINISTRATOR)
            .option(
                sub("cleanup", "Clean up the guild config.")
                    .attach(Cleanup::classic)
                    .option(bool("yes", "Yes.")),
            )
            .option(
                sub("get", "Get a guild config value.")
                    .attach(Get::classic)
                    .option(string("key", "Config key to get.")),
            )
            .option(
                sub("set", "Set a guild config value.")
                    .attach(Set::classic)
                    .option(string("key", "Config key to set."))
                    .option(string("value", "Config value to set.")),
            )
    }

    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Get a guild configuration value.
struct Get;

impl Get {
    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Set a guild configuration value.
struct Set;

impl Set {
    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Clean config from dangling id references and other expired things.
struct Cleanup;

impl Cleanup {
    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}
