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
        .bind(meta::Ping::command())
        .bind(meta::About::command())
        .bind(meta::Help::command());

    #[cfg(feature = "user")]
    commands
        .bind(user::fuel::Fuel::command())
        .bind(user::time::Time::command())
        // .bind(user::quote::Quote::command()) // WIP
        .bind(user::joke::Joke::command())
        .bind(user::coinflip::Coinflip::command())
        .bind(user::user_info::UserInfo::command());

    // #[cfg(feature = "voice")]
    // commands.bind(user::voice::Voice::command()); // WIP

    // Moderation functionality.
    #[cfg(feature = "admin")]
    commands
        // .bind(admin::config::Config::command()) // WIP
        // .bind(admin::alias::Alias::command()) // WIP
        .bind(admin::roles::Roles::command())
        .bind(admin::bot::Bot::command())
        .bind(admin::silence::Mute::command());

    // Extra utility.
    #[cfg(feature = "bulk-delete")]
    commands.bind(admin::bulk::BulkDelete::command());

    // Bot owner functionality.
    #[cfg(feature = "owner")]
    commands.bind(owner::Shutdown::command());

    commands
        .validate()
        .context("Failed to validate commands list")?;

    Ok(commands.build())
}
