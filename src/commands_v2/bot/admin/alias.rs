use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;

/// Command: Manage guild command aliases.
pub struct Alias;

impl Alias {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands_v2::builder::*;

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

    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

/// Command: List guild command aliases.
pub struct List;

impl List {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Get a guild command alias definition.
pub struct Get;

impl Get {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Set a guild command alias definition.
pub struct Set;

impl Set {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Remove a guild command alias definition.
pub struct Remove;

impl Remove {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}
