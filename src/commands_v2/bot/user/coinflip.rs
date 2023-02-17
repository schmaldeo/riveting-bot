use rand::random;

use crate::commands_v2::prelude::*;

/// Command: Coinflip.
pub struct Coinflip {
    args: Args,
}

impl Coinflip {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands_v2::builder::*;

        command("coinflip", "Flip a coin.").attach(Self::slash)
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        let flip = random::<bool>();
        if flip {
            Ok(Response::CreateMessage("Heads".to_string()))
        } else {
            Ok(Response::CreateMessage("Tails".to_string()))
        }
    }
}
