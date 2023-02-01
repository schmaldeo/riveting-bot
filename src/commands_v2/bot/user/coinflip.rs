use rand::random;

use crate::commands_v2::prelude::*;

/// Command: Coinflip.
#[derive(Default)]
pub struct Coinflip {
    args: Args,
}

impl Coinflip {
    pub async fn classic(_ctx: Context, _req: ClassicRequest) -> CommandResult {
        todo!();
    }

    pub async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        let flip = random::<bool>();
        if flip {
            Ok(Response::CreateMessage("Heads".to_string()))
        } else {
            Ok(Response::CreateMessage("Tails".to_string()))
        }
    }
}
