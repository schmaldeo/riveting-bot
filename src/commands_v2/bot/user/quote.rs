use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;

/// Command: Quote a random person or manage quotes.
#[derive(Default)]
pub struct Quote;

impl Quote {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Add a quote.
#[derive(Default)]
pub struct Add;

impl Add {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Remove a quote.
#[derive(Default)]
pub struct Remove;

impl Remove {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

fn quote_user(text: &str, user: &str) -> String {
    format!(">>> {text}\n\t*— {user}*")
}
