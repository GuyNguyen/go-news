use std::{env, process::exit};
use dotenv::dotenv;

use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};

mod feed_reader;

const HELP_MESSAGE: &str = "
    Hello there, Human!

    You have summoned me. Let's see about getting you what you need.

    ? Need technical help?
    => Message @guy0284, and he'll tenuki over

    I hope that resolves your issue!
    -- Go News

    ";

const HELP_COMMAND: &str ="!help";

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == HELP_COMMAND {
            if let Err(why) = msg.channel_id.say(&ctx.http, HELP_MESSAGE).await {
                println!("Error sending message: {why:?}");
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    ctrlc::set_handler(move || {
        println!("Exiting...");
        exit(0)
    })
    .expect("Error setting Ctrl-C handler");

    dotenv().ok();

    let _result = feed_reader::feed_reader().await;

    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token in the environment");

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
