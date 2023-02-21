use crate::commands::prelude::*;
// use crate::utils::prelude::*;

/// Command: Quote a random person or manage quotes.
pub struct Quote;

impl Quote {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands::builder::*;

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

    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResponse {
        todo!();
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResponse {
        todo!();
    }

    async fn message(_ctx: Context, _req: MessageRequest) -> CommandResponse {
        todo!();
    }
}

/// Command: Add a quote.
struct Add;

impl Add {
    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResponse {
        todo!();
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResponse {
        todo!();
    }

    async fn message(_ctx: Context, _req: MessageRequest) -> CommandResponse {
        todo!();
    }
}

/// Command: Remove a quote.
struct Remove;

impl Remove {
    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResponse {
        todo!();
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResponse {
        todo!();
    }

    async fn message(_ctx: Context, _req: MessageRequest) -> CommandResponse {
        todo!();
    }
}

fn quote_user(text: &str, user: &str) -> String {
    format!(">>> {text}\n\t*— {user}*")
}
