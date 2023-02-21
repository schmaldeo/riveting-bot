use crate::commands::prelude::*;
// use crate::utils::prelude::*;

/// Command: Manage guild command aliases.
pub struct Alias;

impl Alias {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands::builder::*;

        command("alias", "Manage guild aliases.")
            .attach(Self::classic)
            .permissions(Permissions::ADMINISTRATOR)
            .option(sub("list", "List guild aliases.").attach(List::classic))
            .option(
                sub("get", "Get a guild alias.")
                    .attach(Get::classic)
                    .option(string("alias", "Get definition by alias name.").required()),
            )
            .option(
                sub("set", "Set a guild alias.")
                    .attach(Set::classic)
                    .option(string("alias", "Alias to set.").required())
                    .option(string("definition", "Alias definition.").required()),
            )
            .option(
                sub("remove", "Delete a guild alias.")
                    .attach(Remove::classic)
                    .option(string("alias", "Alias to delete.").required()),
            )
    }

    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResponse {
        todo!();
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResponse {
        todo!();
    }
}

/// Command: List guild command aliases.
struct List;

impl List {
    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResponse {
        todo!();
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResponse {
        todo!();
    }
}

/// Command: Get a guild command alias definition.
struct Get;

impl Get {
    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResponse {
        todo!();
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResponse {
        todo!();
    }
}

/// Command: Set a guild command alias definition.
struct Set;

impl Set {
    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResponse {
        todo!();
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResponse {
        todo!();
    }
}

/// Command: Remove a guild command alias definition.
struct Remove;

impl Remove {
    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResponse {
        todo!();
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResponse {
        todo!();
    }
}
