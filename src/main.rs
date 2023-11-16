#![allow(unused_imports)]
use poise::serenity_prelude as serenity;
use dotenv::dotenv;
use tokio::time::Duration;
use reqwest::{get, Client, header::USER_AGENT};
use serde_json::Value;
use crate::serenity::{Mention, ChannelId, RoleId};
use serenity::model::channel::Embed;
use chrono::prelude::*;
use botcafe::{html_decode, escpae_markdown, has_error, grab_data};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

mod heycafe;
mod userfeed;
mod cafefeed;

async fn listener(ctx: &serenity::Context, event: &poise::Event<'_>, data: &Data) -> Result<(), Error> {
    match event {
        poise::Event::GuildCreate { guild, .. } => {
            let guild_id = *guild.id.as_u64() as i64;
            let count = sqlx::query!("SELECT COUNT(id) AS count FROM guild_settings WHERE guild_id = ?", guild_id)
                .fetch_one(&data.database)
                .await
                .unwrap();

            if count.count == 0 {
                sqlx::query!("INSERT INTO guild_settings (guild_id, feed_settings_required_roleid) VALUES (?, 0)", guild_id)
                    .execute(&data.database)
                    .await
                    .unwrap();

                println!("[GUILD] Joined new guild: {} (ID: {})", guild.name, guild.id.as_u64());
            }
        },
        poise::Event::Ready { .. } => {
            println!("Bot.Cafe started!");
            feed_check(ctx, data).await?;
        },
        _ => {}
    }

    Ok(())
}

// Hey.Cafe Feed data
#[derive(Debug)]
#[allow(dead_code)]
struct UserFeed {
    id: i64,
    guild_id: i64,
    feed_type: String,
    channel_id: i64,
    heycafe_id: String,
    last_post_id: String,
    mention_role_id: i64,
    tag_id: String,
    last_post_timestamp: i64
}

// Hey.Cafe feeds
async fn feed_check(ctx: &serenity::Context, data: &Data) -> Result<(), Error> {
    loop {
        println!("[{}] Running feed check...", Utc::now().format("%H:%M:%S"));
        let feed_vector: Vec<UserFeed> = sqlx::query_as!(UserFeed, "SELECT * FROM heycafe_feeds")
            .fetch_all(&data.database)
            .await
            .unwrap();

        for feed in feed_vector {
            // Grab data and run checks
            let api_data = match grab_data(&feed.tag_id, &feed.feed_type, &feed.heycafe_id, &data.client).await {
                Some(neat) => neat,
                None => continue,
            };

            if has_error(&api_data) { continue; }

            if &feed.feed_type == "user" && !api_data["response_data"]["conversations"][0]["cafe"].is_boolean() { continue; }

            let new_timestamp = api_data["response_data"]["conversations"][0]["date_created"].as_str().unwrap().parse::<i64>().unwrap();
            if feed.last_post_timestamp > new_timestamp  { continue; }

            let new_id = api_data["response_data"]["conversations"][0]["id"].as_str().unwrap();
            if new_id == feed.last_post_id { continue; }

            // Format message to post
            let channel_id: ChannelId = ChannelId(feed.channel_id as u64);
            
            let embed_author = match feed.feed_type.as_str() {
                "user" => {
                    format!("{} (@{})", 
                        api_data["response_data"]["conversations"][0]["account"]["name"].as_str().unwrap(),
                        api_data["response_data"]["conversations"][0]["account"]["alias"].as_str().unwrap())
                },
                "cafe" => {
                    format!("{} (!{})",
                        api_data["response_data"]["conversations"][0]["cafe"]["name"].as_str().unwrap(),
                        api_data["response_data"]["conversations"][0]["cafe"]["alias"].as_str().unwrap())
                },
                _ => String::new()
            };

            let mut embed_desc = String::from(api_data["response_data"]["conversations"][0]["contents"].as_str().unwrap());
            if embed_desc.chars().count() >= 4096 {
                embed_desc = console::truncate_str(&embed_desc, 4096, "...").to_string();
            }
            embed_desc = html_decode(embed_desc);
            embed_desc = escpae_markdown(embed_desc);

            let tag_info = if &feed.tag_id != "none" {
                format!("{} {}", api_data["response_data"]["conversations"][0]["tag"]["emoji"].as_str().unwrap(), api_data["response_data"]["conversations"][0]["tag"]["name"].as_str().unwrap())
            } else {
                String::new()
            };

            let mut attachment_info = String::new();
            let mut image_url = String::new();
            if !api_data["response_data"]["conversations"][0]["attachments"].is_boolean() {
                attachment_info = String::from("Yes");

                for (_key, value) in api_data["response_data"]["conversations"][0]["attachments"].as_object().unwrap().iter() {
                    if value["type"].as_str().unwrap() == "image" {
                        image_url = value["file"].as_str().unwrap().to_string();
                        break;
                    }
                }
            }

            let mention_text = if feed.mention_role_id != 0 {
                format!("{}", Mention::from(RoleId(feed.mention_role_id as u64)))
            } else {
                String::new()
            };

            // Post content
            let send = channel_id.send_message(&ctx, |m| {
                m.content(mention_text);
                m.embed(|e| {
                    e.color(0x604fd8);
                    e.title(embed_author);
                    e.url(format!("https://hey.cafe/conversation/{new_id}"));
                    e.thumbnail(api_data["response_data"]["conversations"][0]["account"]["avatar"].as_str().unwrap());
                    e.description(embed_desc);
                    if !tag_info.is_empty() {
                        e.field("Tag:", tag_info, true);
                    }
                    if !attachment_info.is_empty() {
                        e.field("Attachments:", attachment_info, true);
                    }
                    if feed.feed_type == "cafe" {
                        e.field("Author:", api_data["response_data"]["conversations"][0]["account"]["name"].as_str().unwrap(), true);
                    }
                    e.image(image_url);
                    e.footer(|f|
                        f.text(format!("Shared to Discord at {}", Utc::now().format("%Y-%m-%d %H:%M:%S")))
                    )
                })
            }).await;

            if let Err(e) = send {
                println!("Failed to post message: {}", e);
                continue;
            }

            sqlx::query!("UPDATE heycafe_feeds SET last_post_id = ?, last_post_timestamp = ? WHERE channel_id = ? AND heycafe_id = ? AND tag_id = ?", new_id, new_timestamp, feed.channel_id, feed.heycafe_id, feed.tag_id)
                .execute(&data.database)
                .await
                .unwrap();
            
            println!("NEW POST - Guild: {} - Channel: {} - Post ID: {}", feed.guild_id, feed.channel_id, new_id);
        }

        tokio::time::sleep(Duration::from_secs(30)).await;
    }

    #[allow(unreachable_code)]
    Ok(())
}

#[derive(Debug)]
pub struct Data { // User data, which is stored and accessible in all command invocations
    database: sqlx::SqlitePool,
    client: reqwest::Client
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Connect to sqlite DB
    let database_url = std::env::var("DATABASE_URL").expect("missing DATABASE_URL");
    let database = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            database_url
                .parse::<sqlx::sqlite::SqliteConnectOptions>().unwrap()
                .create_if_missing(true),
        )
        .await.unwrap();
    //sqlx::migrate!("./migrations").run(&database).await.unwrap();

    // Bulid Client
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko)")
        .build()
        .unwrap();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                userfeed::userfeed(),
                cafefeed::cafefeed(),
                heycafe::listfeeds(),
                heycafe::hey(),
                heycafe::botcafe()
            ],
            event_handler: |ctx, event, _, data| Box::pin(listener(ctx, event, data)),
            ..Default::default()
        })
        .token(std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN"))
        .intents(serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::GUILDS)
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    database,
                    client,
                })
            })
        });

    framework.run().await.unwrap();
}
