use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;

/// Command: Voice channel controls.
pub struct Voice;

impl Voice {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands_v2::builder::*;

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

    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Tell the bot to connect to a voice channel.
pub struct Join;

impl Join {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}

/// Command: Tell the bot to disconnect from a voice channel.
pub struct Leave;

impl Leave {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        todo!();
    }
}
