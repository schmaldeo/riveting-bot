// use twilight_model::channel::ChannelType;
use twilight_model::guild::Permissions;

use crate::commands_v2::{Command, CommandsBuilder};
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

pub fn create_commands() -> AnyResult<CommandsBuilder> {
    use crate::commands_v2::builder::*;

    let mut commands = CommandsBuilder::new();

    // TODO: Add impls and command details, permissions, args, etc.

    // Basic functionality.
    commands
        .bind(
            command("ping", "Ping the bot.")
                .attach(meta::Ping::classic)
                .attach(meta::Ping::slash)
                .dm(),
        )
        .bind(
            command("about", "Display info about the bot.")
                .attach(meta::About::classic)
                .attach(meta::About::slash)
                .dm(),
        )
        .bind(
            command("help", "List bot commands.")
                .attach(meta::Help::classic)
                .attach(meta::Help::slash)
                .dm(),
        );

    commands
        .bind(
            command("fuel", "Calculate race fuel required.")
                .attach(user::fuel::Fuel::classic)
                .attach(user::fuel::Fuel::slash)
                .dm(),
        )
        .bind(
            command("time", "Display a discord timestamp.")
                .attach(user::time::Time::classic)
                .attach(user::time::Time::slash)
                .dm(),
        );

    #[cfg(feature = "voice")]
    commands.bind(
        command("voice", "Manage voice connection.")
            .attach(user::voice::Voice::classic)
            .attach(user::voice::Voice::slash)
            .option(
                sub("join", "Join the bot to a voice channel.")
                    .attach(user::voice::Join::classic)
                    .attach(user::voice::Join::slash),
            )
            .option(
                sub("leave", "Disconnect the bot from a voice channel.")
                    .attach(user::voice::Leave::classic)
                    .attach(user::voice::Leave::slash),
            ),
    );

    // Moderation functionality.
    #[cfg(feature = "admin")]
    commands
        .bind(
            command("config", "Manage guild config.")
                .attach(admin::config::Config::classic)
                .permissions(Permissions::ADMINISTRATOR)
                .option(
                    sub("cleanup", "Clean up the guild config.")
                        .attach(admin::config::Cleanup::classic)
                        .option(bool("yes", "Yes.")),
                )
                .option(
                    sub("get", "Get a guild config value.")
                        .attach(admin::config::Get::classic)
                        .option(string("key", "Config key to get.")),
                )
                .option(
                    sub("set", "Set a guild config value.")
                        .attach(admin::config::Set::classic)
                        .option(string("key", "Config key to set."))
                        .option(string("value", "Config value to set.")),
                ),
        )
        .bind(
            command("alias", "Manage guild aliases.")
                .attach(admin::alias::Alias::classic)
                .permissions(Permissions::ADMINISTRATOR)
                .option(sub("list", "List guild aliases.").attach(admin::alias::List::classic))
                .option(
                    sub("get", "Get a guild alias.")
                        .attach(admin::alias::Get::classic)
                        .option(string("alias", "Get definition by alias name.").required()),
                )
                .option(
                    sub("set", "Set a guild alias.")
                        .attach(admin::alias::Set::classic)
                        .option(string("alias", "Alias to set.").required())
                        .option(string("definition", "Alias definition.").required()),
                )
                .option(
                    sub("remove", "Delete a guild alias.")
                        .attach(admin::alias::Remove::classic)
                        .option(string("alias", "Alias to delete.").required()),
                ),
        )
        .bind(
            command("roles", "Manage reaction-roles.")
                .attach(admin::roles::Roles::classic)
                .attach(admin::roles::Roles::slash)
                .permissions(Permissions::ADMINISTRATOR)
                .option(sub("setup", "Setup a new reaction-roles message."))
                .option(sub("edit", "Edit an existing reaction-roles message.")),
        )
        .bind(
            command("bot", "Create or edit bot messages.")
                .attach(admin::bot::Bot::classic)
                .attach(admin::bot::Bot::slash)
                .permissions(Permissions::ADMINISTRATOR)
                .option(sub("say", "Post a message by the bot."))
                .option(sub("edit", "Edit an existing bot message.")),
        )
        .bind(
            command("mute", "Silence someone in voice channel.")
                .attach(admin::silence::Mute::classic)
                .attach(admin::silence::Mute::slash)
                .attach(admin::silence::Mute::user)
                .permissions(Permissions::ADMINISTRATOR),
        )
        .bind(
            command("timeout", "Give someone a timeout.")
                .attach(admin::silence::Timeout::classic)
                .attach(admin::silence::Timeout::slash)
                .attach(admin::silence::Timeout::message)
                .attach(admin::silence::Timeout::user)
                .permissions(Permissions::ADMINISTRATOR),
        );

    // Extra utility.
    #[cfg(feature = "bulk-delete")]
    commands.bind(
        command("bulk-delete", "Delete many of messages.")
            .attach(admin::bulk::BulkDelete::classic)
            .attach(admin::bulk::BulkDelete::slash)
            .permissions(Permissions::ADMINISTRATOR),
    );

    // Bot owner functionality.
    #[cfg(feature = "owner")]
    commands.bind(
        command("shutdown", "Shutdown the bot.")
            .attach(owner::Shutdown::classic)
            .permissions(Permissions::MANAGE_GUILD)
            .dm(),
    );

    commands.validate().unwrap();

    Ok(commands)
}
