// Used for miscellaneous commands

use crate::{UserFeed, Context, Error};
use poise::serenity_prelude as serenity;
use reqwest::{get, Client, header::USER_AGENT};
use serde_json::Value;
use serenity::{ChannelId, RoleId};

// COMMAND - /listfeeds
/// Lists all feeds set for this server.
#[poise::command(
    slash_command,
    required_permissions = "MANAGE_CHANNELS",
)]
pub async fn listfeeds(
    ctx: Context<'_>,
    #[description = "Type \"user\" or \"cafe\" for the type of feeds to list."] feed_type: String
) -> Result<(), Error> {
    let guild_id = *ctx.guild_id().unwrap().as_u64() as i64;

    // Grab feeds
    if feed_type.as_str() != "user" && feed_type.as_str() != "cafe" {
        let msg = format!("{}, you can only choose \"cafe\" or \"user\" as the feed type!", ctx.author());
        ctx.say(msg).await?;
        return Ok(());
    }

    let server_feeds: Vec<UserFeed> = sqlx::query_as!(UserFeed, "SELECT * FROM heycafe_feeds WHERE guild_id = ? AND feed_type = ?", guild_id, feed_type)
        .fetch_all(&ctx.data().database)
        .await
        .unwrap();

    if server_feeds.is_empty() {
        let msg = format!("{}, no feeds found for this server!", ctx.author());
        ctx.say(msg).await?;
        return Ok(());
    }

    let ftype =  if feed_type.as_str() == "user" { "account" } else { "cafe" };

    // Feeds to text
    let mut feed_display = String::new();
    for feed in server_feeds {
        let api_link = format!("https://endpoint.hey.cafe/api/{ftype}_info?query={}", feed.heycafe_id);

        let api_info_request = ctx.data().client.get(api_link.clone())
            .send()
            .await;
        
        let api_info_request = match api_info_request {
            Ok(ok) => ok,
            Err(err) => {
                if err.is_timeout() {
                    println!("Timeout");
                    break;
                } else {
                    println!("{err}");
                    break;
                }
            }
        };

        let api_info = api_info_request
            .json::<serde_json::Value>()
            .await?;

        let channel_id = ChannelId(feed.channel_id as u64);

        let display_name = api_info["response_data"]["name"].as_str().unwrap();
        let alias = api_info["response_data"]["alias"].as_str().unwrap();

        let prefix = if feed_type.as_str() == "user" { "@" } else { "!" };

        let tag_name = if feed.tag_id != "none" {
            format!("{} {}", api_info["response_data"]["tags"][&feed.tag_id]["emoji"].as_str().unwrap(), api_info["response_data"]["tags"][&feed.tag_id]["name"].as_str().unwrap())
        } else {
            String::from("None")
        };

        let role_name = if feed.mention_role_id != 0 {
            let role_id = RoleId(feed.mention_role_id as u64);

            role_id.to_role_cached(ctx).unwrap().name
        } else {
            String::from("None")
        };

        feed_display = format!("{feed_display}- Name: {display_name}({prefix}{alias}) - Channel: <#{channel_id}> - Tag: {tag_name} - Mentions: {role_name}\n");
    }

    let title = if feed_type.as_str() == "user" { "__**User Feeds**__" } else { "__**Cafe Feeds**__" };
    let msg = format!("{title}\n{feed_display}");
    ctx.say(msg).await?;
    println!("[LOG] COMMAND: /listfeeds {} - Guild {}({})", feed_type, ctx.guild().unwrap().name, guild_id);

    Ok(())
}

// COMMAND - /hey
/// Links to Hey.Cafe
#[poise::command(slash_command)]
pub async fn hey(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(|m| {
        m.embed(|e| {
            e.title("Visit Hey.Café!");
            e.url("https://hey.cafe");
            e.thumbnail("https://hey.cafe/logo.png");
            e.description("Hey.Café is a new social network designed to be easy to use. When you join you can create new conversations and join communities that we call cafés based on your interests, and it's free to use.")
        })
    }).await?;
    let guild_id = *ctx.guild_id().unwrap().as_u64() as i64;
    println!("[LOG] COMMAND: /hey - Guild: {}({})", ctx.guild().unwrap().name, guild_id);

    Ok(())
}