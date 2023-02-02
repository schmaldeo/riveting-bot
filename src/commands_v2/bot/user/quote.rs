use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;

/// Command: Quote a random person or manage quotes.
pub struct Quote;

impl Quote {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands_v2::builder::*;

        command("quote", "Get a random quote.")
            .attach(Self::classic)
            .attach(Self::slash)
            .attach(Self::message)
            .option(
                sub("add", "Create a quote.")
                    .attach(Add::classic)
                    .attach(Add::slash),
            )
            .option(
                sub("remove", "Delete a quote.")
                    .attach(Remove::classic)
                    .attach(Remove::slash),
            )
    }

    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }

    pub async fn message(_ctx: Context, _req: MessageRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Add a quote.
pub struct Add;

impl Add {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }

    pub async fn message(_ctx: Context, _req: MessageRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Remove a quote.
pub struct Remove;

impl Remove {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }

    pub async fn message(_ctx: Context, _req: MessageRequest) -> CommandResult {
        todo!();
    }
}

fn quote_user(text: &str, user: &str) -> String {
    format!(">>> {text}\n\t*— {user}*")
}
