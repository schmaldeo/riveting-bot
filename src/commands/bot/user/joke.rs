use reqwest;

use crate::commands::prelude::*;

/// Command: Send a dad joke.
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

impl Joke {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands::builder::*;

        command("joke", "Send a bad joke.").attach(Self::slash)
    }

    async fn slash(_ctx: Context, _req: SlashRequest) -> CommandResult {
        let body = reqwest::get("https://v2.jokeapi.dev/joke/Any")
            .await?
            .json::<JokeResponse>()
            .await?;

        let joke = match body {
            JokeResponse::Single { joke } => joke,
            JokeResponse::TwoPart { setup, delivery } => format!("> {setup}\n> {delivery}"),
        };

        Ok(Response::CreateMessage(joke))
    }
}
