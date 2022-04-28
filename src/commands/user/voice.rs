use std::sync::Arc;

use twilight_model::id::Id;

use crate::commands::{CommandContext, CommandError, CommandResult};
use crate::utils::*;

/// Command: Voice channel controls.
pub async fn voice(cc: CommandContext<'_>) -> CommandResult {
    // TODO Display help.
    if !cc.args.is_empty() {
        return Err(CommandError::NotImplemented);
    }
    Ok(())
}

/// Command: Tell the bot to connect to a voice channel.
pub async fn join(cc: CommandContext<'_>) -> CommandResult {
    let mut call = songbird::Call::new(
        songbird::id::GuildId(cc.msg.guild_id.unwrap().get()),
        songbird::shards::Shard::TwilightCluster(Arc::clone(&cc.cluster), cc.shard.unwrap()),
        cc.user.id,
    );

    call.join(Id::new(674618888750891049)).await.unwrap();

    // let file = std::fs::File::open("./test.wav").unwrap();
    // let reader = songbird::input::Reader::from_file(file);
    // let kind = songbird::input::Codec::Pcm;
    // let container = songbird::input::Container::Raw;
    // let source = songbird::input::Input::new(false, reader, kind, container, None);
    // let (tx, rx) = flume::unbounded(); // rx: flume::Receiver<songbird::tracks::TrackCommand>;
    // let uuid: uuid::Uuid;
    // let meta=songbird::input::Metadata::from_ffprobe_json(value);
    // let metadata: Box<songbird::input::Metadata>;
    // let handle = songbird::tracks::TrackHandle::new(tx, false, uuid, metadata);
    // let handle: songbird::tracks::TrackHandle;
    // call.play(songbird::tracks::Track::new_raw(source, rx, handle));

    tokio::time::sleep(std::time::Duration::from_secs(60)).await;

    call.leave().await.unwrap();

    Ok(())
}

/// Command: Tell the bot to disconnect from a voice channel.
pub async fn leave(cc: CommandContext<'_>) -> CommandResult {
    Ok(())
}
