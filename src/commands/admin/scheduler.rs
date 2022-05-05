use std::io::Write;
use std::time::Duration;
use std::{env, fs};

use chrono::{DateTime, TimeZone, Utc};
use delay_timer::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::time::{self, sleep};
use twilight_mention::Mention;
use twilight_model::id::marker::RoleMarker;
use twilight_model::id::Id;
use twilight_util::builder::embed;

use crate::commands::{CommandContext, CommandResult};
use crate::utils::prelude::*;
use crate::Context;

pub async fn scheduler(cc: CommandContext<'_>) -> CommandResult {
    cc.http
        .create_message(cc.msg.channel_id)
        .content("Do `!scheduler add` to add a scheduled event")?
        .send()
        .await?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct Event {
    id: u32,
    name: String,
    added_by: String,
    added_at: DateTime<Utc>,
    finishing_at: DateTime<Utc>,
}

/// command: !scheduler add <name of event> <year> <month> <day> <hours> <minutes> <seconds>
pub async fn add(cc: CommandContext<'_>) -> CommandResult {
    let args: Vec<&str> = cc.args.split(' ').collect();

    let mut date_vec: Vec<u32> = Vec::new();

    for i in 1..7 {
        match args[i].parse::<u32>() {
            Ok(n) => date_vec.push(n),
            Err(e) => {
                cc.http
                    .create_message(cc.msg.channel_id)
                    .reply(cc.msg.id)
                    .content(&e.to_string())?
                    .send()
                    .await?;
                break;
            },
        }
    }

    let time = Utc::now();
    let completion = Utc
        .ymd(date_vec[0] as i32, date_vec[1], date_vec[2])
        .and_hms(date_vec[3], date_vec[4], date_vec[5]);

    let query_user: String = cc.msg.id.get().to_string();

    let rand_file_name: u32 = rand::thread_rng().gen();

    let event = Event {
        id: rand_file_name,
        name: args[0].to_string(),
        added_by: query_user,
        added_at: time,
        finishing_at: completion,
    };

    let serialised_event: String = serde_json::to_string(&event).unwrap();

    fs::File::create(format!("./data/events/{}.json", rand_file_name))
        .map_err(|e| anyhow::anyhow!("Failed to create a file: {}", e))?;

    fs::create_dir_all("./data/events")
        .map_err(|e| anyhow::anyhow!("Failed to create events dir: {}", e))?;

    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(format!("./data/events/{}.json", rand_file_name))
        .unwrap();

    write!(file, "{}", serialised_event)
        .map_err(|e| anyhow::anyhow!("Failed to write to file: {}", e))?;

    let msg = format!(
        "```Event added!\nEvent's name: {}\nEvent's ID: {},\nFinish date: {}```",
        &args[0], &rand_file_name, &completion
    );

    cc.http
        .create_message(cc.msg.channel_id)
        .reply(cc.msg.id)
        .content(&msg)?
        .send()
        .await?;

    Ok(())
}

pub async fn rm(cc: CommandContext<'_>) -> CommandResult {
    let args: Vec<&str> = cc.args.split(' ').collect();
    if args[0].is_empty() {
        cc.http
            .create_message(cc.msg.channel_id)
            .reply(cc.msg.id)
            .content("Specify event ID")?
            .send()
            .await?;
    } else {
        fs::remove_file(format!("./data/events/{}.json", args[0]))
            .map_err(|e| anyhow::anyhow!("Failed to remove the file: {}", e))?;
        cc.http
            .create_message(cc.msg.channel_id)
            .reply(cc.msg.id)
            .content("Event removed")?
            .send()
            .await?;
    }

    Ok(())
}

pub async fn handle_timer(cc: Context) -> AnyResult<()> {
    async fn interval_loop(cc: &Context) {
        let paths = fs::read_dir("./data/events").unwrap();

        let now: i64 = DateTime::timestamp(&Utc::now());
        let mut tasks: Vec<Event> = Vec::new();

        for path in paths.into_iter() {
            let current_file: &String = &path.unwrap().path().display().to_string();
            let string: String =
                String::from_utf8_lossy(&fs::read(&current_file).expect("Can't load the file"))
                    .parse()
                    .expect("Can't parse the file");
            let event: Event = serde_json::from_str(&string).unwrap();
            let finish_time: i64 = DateTime::timestamp(&event.finishing_at);
            if (finish_time - now) < 3600 {
                tasks.push(event);
            };
        }
        for task in tasks.into_iter() {
            let finish_time: i64 = DateTime::timestamp(&task.finishing_at);
            let time_left: i64 = finish_time - now;
            sleep(Duration::from_secs(time_left as u64)).await;
            fs::remove_file(format!("./data/events/{}.json", &task.id)).expect("msg");

            let embed = embed::EmbedBuilder::new()
                .title(format!("{} is starting", &task.name))
                .description(format!("Starts at: {}", &task.finishing_at))
                .color(0xed00fa)
                .build();
            let role_id = Id::<RoleMarker>::new(
                env::var("ANNOUNCEMENT_CHANNEL")
                    .expect("no channel defined")
                    .parse()
                    .expect("no channel lol"),
            );
            let message = format!("{}", role_id.mention());

            cc.http
                .create_message(Id::new(
                    env::var("ANNOUNCEMENT_CHANNEL")
                        .unwrap()
                        .parse()
                        .expect("fdsafdsa"),
                ))
                .content(&message)
                .expect("msg")
                .embeds(&[embed])
                .unwrap()
                .send()
                .await
                .expect("msg");
        }
        println!("lol")
    }

    let mut interval = time::interval(time::Duration::from_secs(3600));
    loop {
        interval.tick().await;
        interval_loop(&cc).await;
    }
    Ok(())
}
