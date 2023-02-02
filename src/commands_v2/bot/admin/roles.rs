use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;

pub struct Roles;

impl Roles {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands_v2::builder::*;

        command("roles", "Manage reaction-roles.")
            .attach(Self::classic)
            .attach(Self::slash)
            .permissions(Permissions::ADMINISTRATOR)
            .option(sub("setup", "Setup a new reaction-roles message."))
            .option(sub("edit", "Edit an existing reaction-roles message."))
    }

    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}
