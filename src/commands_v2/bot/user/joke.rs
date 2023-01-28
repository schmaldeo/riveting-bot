use reqwest;

use crate::commands_v2::prelude::*;

/// Command: Send a dad joke.
#[derive(Default)]
pub struct Joke {
    args: Args,
}

#[derive(serde::Deserialize, Debug)]
struct JokeResponse {
    setup: String,
    punchline: String,
}

impl Command for Joke {
    type Data = Self;

    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        Ok(Response::Clear)
    }

    async fn slash(_ctx: Context, req: SlashRequest) -> CommandResult {
        let joke_type = if let Some(req_type) = req.args.get("type").string() {
            req_type.to_string()
        } else {
            "general".to_string()
        };

        let url = format!("https://official-joke-api.appspot.com/jokes/{joke_type}/random");
        println!("{url}");
        let body = reqwest::get(url)
            .await
            .unwrap()
            .json::<Vec<JokeResponse>>()
            .await
            .unwrap();
        let joke = format!("> {}\n> {}", body[0].setup, body[0].punchline);
        Ok(Response::CreateMessage(joke))
    }
}
