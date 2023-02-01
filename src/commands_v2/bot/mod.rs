use crate::commands_v2::{Command, Commands, CommandsBuilder};
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
                .option(string("command", "Get help on a command."))
                .dm(),
        );

    commands
        .bind(
            command("fuel", "Calculate race fuel required.")
                .attach(user::fuel::Fuel::slash)
                .option(
                    integer("length", "Lenght of the race in minutes")
                        .required()
                        .min(1),
                )
                .option(
                    integer("minutes", "Minutes in the lap")
                        .required()
                        .min(0)
                        .max(10),
                )
                .option(
                    number(
                        "seconds",
                        "Seconds (and optionally milliseconds after a comma) in the lap",
                    )
                    .required()
                    .min(0.0)
                    .max(59.9999),
                )
                .option(
                    number("fuel_consumption", "Fuel consumption in l/lap")
                        .required()
                        .min(0.1),
                )
                .dm(),
        )
        .bind(
            command("time", "Display a discord timestamp.")
                .attach(user::time::Time::classic)
                .attach(user::time::Time::slash)
                .option(string("expression", "Time expression to evaluate."))
                .dm(),
        )
        .bind(command("joke", "Send a bad joke.").attach(user::joke::Joke::slash))
        .bind(command("coinflip", "Coin flip").attach(user::coinflip::Coinflip::slash))
        .bind(
            command("userinfo", "Get information about a user")
                .attach(user::user_info::UserInfo::slash)
                .option(user("user", "Mention a user")),
        );

    #[cfg(feature = "voice")]
    commands.bind(
        command("voice", "Manage voice connection.")
            .attach(user::voice::Voice::classic)
            .attach(user::voice::Voice::slash)
            .option(
                sub("join", "Join the bot to a voice channel.")
                    .attach(user::voice::Join::classic)
                    .attach(user::voice::Join::slash)
                    .option(
                        channel("channel", "Voice channel to join.")
                            .required()
                            .types([ChannelType::GuildVoice, ChannelType::GuildStageVoice]),
                    ),
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
                .option(
                    sub("say", "Post a message by the bot.")
                        .attach(admin::bot::Say::classic)
                        .attach(admin::bot::Say::slash)
                        .option(string("text", "What to say.").required())
                        .option(channel("channel", "Where to send it.")),
                )
                .option(
                    sub("edit", "Edit an existing bot message.")
                        .attach(admin::bot::Edit::classic)
                        .attach(admin::bot::Edit::slash)
                        .option(message("message", "Message to edit.").required()),
                ),
        )
        .bind(
            command("mute", "Silence someone in voice channel.")
                .attach(admin::silence::Mute::classic)
                .attach(admin::silence::Mute::slash)
                .attach(admin::silence::Mute::user)
                .permissions(Permissions::ADMINISTRATOR)
                .option(user("user", "Who to mute.").required())
                .option(integer("seconds", "Duration of the mute.").min(0)),
        );

    // Extra utility.
    #[cfg(feature = "bulk-delete")]
    commands.bind(
        command("bulk-delete", "Delete many of messages.")
            .attach(admin::bulk::BulkDelete::classic)
            .attach(admin::bulk::BulkDelete::slash)
            .permissions(Permissions::ADMINISTRATOR)
            .option(
                integer("amount", "Number of messages to delete.")
                    .required()
                    .min(0)
                    .max(100),
            ),
    );

    // Bot owner functionality.
    #[cfg(feature = "owner")]
    commands.bind(
        command("shutdown", "Shutdown the bot.")
            .attach(owner::Shutdown::classic)
            .permissions(Permissions::MANAGE_GUILD)
            .dm(),
    );

    commands
        .validate()
        .context("Failed to validate commands list")?;

    Ok(commands.build())
}
