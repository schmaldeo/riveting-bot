use crate::commands::prelude::*;
// use crate::utils::prelude::*;

/// Command: Voice channel controls.
pub struct Voice;

impl Voice {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands::builder::*;

        command("voice", "Manage voice connection.")
            .attach(Self::classic)
            .attach(Self::slash)
            .option(
                sub("join", "Join the bot to a voice channel.")
                    .attach(Join::classic)
                    .attach(Join::slash)
                    .option(
                        channel("channel", "Voice channel to join.")
                            .required()
                            .types([ChannelType::GuildVoice, ChannelType::GuildStageVoice]),
                    ),
            )
            .option(
                sub("leave", "Disconnect the bot from a voice channel.")
                    .attach(Leave::classic)
                    .attach(Leave::slash),
            )
    }

    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Tell the bot to connect to a voice channel.
struct Join;

impl Join {
    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Tell the bot to disconnect from a voice channel.
struct Leave;

impl Leave {
    async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}
