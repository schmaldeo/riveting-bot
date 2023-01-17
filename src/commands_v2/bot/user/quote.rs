use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;

/// Command: Quote a random person or manage quotes.
#[derive(Default)]
pub struct Quote;

impl Command for Quote {
    type Data = Self;

    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        todo!()
    }
}

/// Command: Add a quote.
#[derive(Default)]
pub struct Add;

impl Command for Add {
    type Data = Self;

    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        todo!()
    }
}

/// Command: Remove a quote.
#[derive(Default)]
pub struct Remove;

impl Command for Remove {
    type Data = Self;

    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        todo!()
    }
}

fn quote_user(text: &str, user: &str) -> String {
    format!(">>> {text}\n\t*— {user}*")
}
