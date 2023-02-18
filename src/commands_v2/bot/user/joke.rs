use reqwest;

use crate::commands_v2::prelude::*;

/// Command: Send a dad joke.
#[derive(Default)]
pub struct Joke {
    args: Args,
}

#[derive(serde::Deserialize)]
#[serde(tag = "type")]
enum JokeResponse {
    #[serde(rename = "single")]
    Single { joke: String },
    #[serde(rename = "twopart")]
    TwoPart { setup: String, delivery: String },
}

impl Command for Joke {
    type Data = Self;

    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        Ok(Response::Clear)
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        let body = reqwest::get("https://v2.jokeapi.dev/joke/Any")
            .await
            .unwrap()
            .json::<JokeResponse>()
            .await;

        let joke = match body {
            Ok(res) => match res {
                JokeResponse::Single { joke } => joke,
                JokeResponse::TwoPart { setup, delivery } => format!("> {setup}\n> {delivery}"),
            },
            Err(_) => "There's been an error".to_string(),
        };

        Ok(Response::CreateMessage(joke))
    }
}
