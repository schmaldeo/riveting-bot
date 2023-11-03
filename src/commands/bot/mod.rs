/*!
Command template:
```
pub struct Command;

impl Command {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands::builder::*;

        command("cmd", "Thing.")
            .attach(Self::classic)
            .attach(Self::slash)
            .attach(Self::message)
            .attach(Self::user)
    }

    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }

    async fn message(_ctx: Context, _req: MessageRequest) -> CommandResult {
        todo!();
    }

    async fn user(_ctx: Context, _req: UserRequest) -> CommandResult {
        todo!();
    }
}
```
*/

use crate::commands::{Commands, CommandsBuilder};
use crate::utils::prelude::*;

/// Generic commands.
pub mod meta;

/// Normal user commands.
#[cfg(feature = "user")]
pub mod user;

/// Administrator comands.
#[cfg(feature = "admin")]
pub mod admin;

/// Bot owner only commands.
#[cfg(feature = "owner")]
pub mod owner;

/// Create the list of bot commands.
pub fn create_commands() -> AnyResult<Commands> {
    let mut commands = CommandsBuilder::new();

    // Basic functionality.
    commands
        .bind(meta::essential::Ping::command())
        .bind(meta::essential::About::command())
        .bind(meta::essential::Help::command());

    #[cfg(feature = "voice")]
    commands.bind(meta::voice::Voice::command());

    // Extra utility.
    #[cfg(feature = "bulk-delete")]
    commands.bind(meta::bulk::BulkDelete::command());

    #[cfg(feature = "user")]
    commands
        .bind(user::fuel::Fuel::command())
        .bind(user::time::Time::command())
        .bind(user::joke::Joke::command())
        .bind(user::coinflip::Coinflip::command())
        .bind(user::battleships::Battleships::command())
        .bind(user::user_info::UserInfo::command());

    // Moderation functionality.
    #[cfg(feature = "admin")]
    commands
        .bind(admin::bot::Bot::command())
        .bind(admin::roles::Roles::command())
        .bind(admin::silence::Mute::command());

    // Bot owner functionality.
    #[cfg(feature = "owner")]
    commands.bind(owner::Shutdown::command());

    commands
        .validate()
        .context("Failed to validate commands list")?;

    Ok(commands.build())
}
