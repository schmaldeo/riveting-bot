/*!
Command template:
```
pub struct Command;

impl Command {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands_v2::builder::*;

        command("cmd", "Thing.")
            .attach(Self::classic)
            .attach(Self::slash)
            .attach(Self::message)
            .attach(Self::user)
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

    pub async fn user(_ctx: Context, _req: UserRequest) -> CommandResult {
        todo!();
    }
}
```
*/
use crate::commands_v2::{Commands, CommandsBuilder};
use crate::utils::prelude::*;

/// Generic commands.
pub mod meta;

/// Normal user commands.
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

    commands
        .bind(user::fuel::Fuel::command())
        .bind(user::time::Time::command())
        .bind(user::quote::Quote::command())
        .bind(user::joke::Joke::command())
        .bind(user::coinflip::Coinflip::command())
        .bind(user::user_info::UserInfo::command());

    #[cfg(feature = "voice")]
    commands.bind(user::voice::Voice::command());

    // Moderation functionality.
    #[cfg(feature = "admin")]
    commands
        .bind(admin::config::Config::command())
        .bind(admin::alias::Alias::command())
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
