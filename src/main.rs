use std::{collections::HashMap, sync::Arc};

use regex::Regex;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{channel::Message, event::MessageUpdateEvent, id::MessageId},
    prelude::{GatewayIntents, RwLock, TypeMapKey},
    Client,
};

#[macro_use]
extern crate lazy_static;

struct AwaitingEditMessages;

impl TypeMapKey for AwaitingEditMessages {
    type Value = Arc<RwLock<HashMap<MessageId, MessageId>>>;
}

struct Handler;

lazy_static! {
    static ref MEDIA_LINK_REGEX: Regex = {
        Regex::new(r"https?://media\.discordapp\.net/attachments/\d{18,19}/\d{18,19}/\S*")
            .expect("Error unwrapping media link regex")
    };
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot || msg.is_private() {
            return;
        }

        let mut fixed_links: Vec<String> = vec![];

        for _match in MEDIA_LINK_REGEX.find_iter(&msg.content) {
            let link = _match.as_str().to_owned();
            println!("Found match in [{}]: {}", msg.id, link);
            fixed_links.push(link.replacen("http://", "https://", 1).replacen(
                "media.discordapp.net",
                "cdn.discordapp.com",
                1,
            ));
        }

        if fixed_links.is_empty() {
            return;
        }

        let mut response = "**✨ Fixed your links!**".to_owned();

        for link in fixed_links {
            response.push('\n');
            response.push_str(&link);
        }

        response
            .push_str("\n\n🍿 Please update the link(s) in your message and I'll delete this one :)\n⁉ What is this?: <https://sexnine.xyz/whycdn>");

        let response_msg: Message = match msg.reply_ping(&ctx, response).await {
            Ok(x) => x,
            Err(e) => return eprintln!("Error replying to user: {e}"),
        };

        let data_lock = {
            let data_read = ctx.data.read().await;

            data_read
                .get::<AwaitingEditMessages>()
                .expect("Failed to get AwaitingEditMessages state")
                .clone()
        };

        let mut sussy = data_lock.write().await;

        sussy.insert(msg.id, response_msg.id);
    }

    async fn message_update(&self, ctx: Context, event: MessageUpdateEvent) {
        let data_lock = {
            let data_read = ctx.data.read().await;

            data_read
                .get::<AwaitingEditMessages>()
                .expect("Failed to get AwaitingEditMessages state")
                .clone()
        };

        let response_msg_id = {
            let sussy = data_lock.read().await;

            match sussy.get(&event.id) {
                Some(x) => x.clone(),
                _ => return,
            }
        };

        let msg = match ctx
            .http
            .get_message(*event.channel_id.as_u64(), *event.id.as_u64())
            .await
        {
            Ok(x) => x,
            Err(e) => {
                eprintln!("Failed to fetch message [{}]: {}", event.id, e);
                return;
            }
        };

        if MEDIA_LINK_REGEX.is_match(msg.content.as_str()) {
            return;
        }

        if let Err(e) = ctx
            .http
            .delete_message(*event.channel_id.as_u64(), *response_msg_id.as_u64())
            .await
        {
            eprintln!("Error while deleting message [{}]: {}", response_msg_id, e)
        }

        let mut sussy = data_lock.write().await;

        sussy.remove(&response_msg_id);
    }
}

#[tokio::main]
async fn main() {
    println!("Bot starting...");

    let token =
        std::env::var("CDN_PLS_TOKEN").expect("CDN_PLS_TOKEN environment variable not set!");

    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    {
        let mut data = client.data.write().await;

        data.insert::<AwaitingEditMessages>(Arc::new(RwLock::new(HashMap::new())));
    }

    if let Err(e) = client.start().await {
        eprintln!("Error: {:?}", e);
    }
}
