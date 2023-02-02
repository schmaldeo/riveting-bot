use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;

/// Command: Create or edit bot messages.
pub struct Bot;

impl Bot {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands_v2::builder::*;

        command("bot", "Create or edit bot messages.")
            .attach(Self::classic)
            .attach(Self::slash)
            .permissions(Permissions::ADMINISTRATOR)
            .option(
                sub("say", "Post a message by the bot.")
                    .attach(Say::classic)
                    .attach(Say::slash)
                    .option(string("text", "What to say.").required())
                    .option(channel("channel", "Where to send it.")),
            )
            .option(
                sub("edit", "Edit an existing bot message.")
                    .attach(Edit::classic)
                    .attach(Edit::slash)
                    .option(message("message", "Message to edit.").required()),
            )
    }

    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Post a message as the bot.
pub struct Say;

impl Say {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Edit a message created by the bot (can be anything).
pub struct Edit;

impl Edit {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}
