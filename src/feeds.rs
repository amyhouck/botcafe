use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use serde_json::Value;
use botcafe::grab_feed_data;

// PARENT
#[poise::command(
    slash_command,
    subcommands("add", "remove")
)]
pub async fn feed(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Add a feed to listen to.
#[poise::command(
    slash_command,
    required_permissions = "MANAGE_CHANNELS",
)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "Alias of the user or cafe."]
    #[max_length = 30] mut alias: String,

    #[description = "Channel to post feeds."]
    #[rename = "channel"] feed_channel: serenity::Channel,

    #[description = "Role to tag in posts."]
    #[rename = "tagrole"] feed_role: Option<serenity::Role>,

    #[description = "Specific user/cafe tag to pull posts from."]
    #[rename = "tag"]
    #[max_length = 30] heycafe_tag: Option<String>
) -> Result<(), Error> {
    // Analyze alias for type and grab data
    let api_feed_type = match alias.chars().nth(0).unwrap() {
        '!' => {
            alias = alias.strip_prefix('!').unwrap().to_string();
            "cafe_info"
        },
        _ => {
            alias = alias.strip_prefix('@').unwrap_or(&alias).to_string();
            "account_info"
        }
    };
    
    let heycafe_data = match grab_feed_data(format!("https://endpoint.hey.cafe/api/{api_feed_type}?query={alias}&convert_numeric=tags"), &ctx.data().client).await {
        Ok(data) => data,
        Err(err) => return Err(err)
    };

    // Validate other args and get necessary info
    let heycafe_id = heycafe_data["response_data"]["id"].as_str().unwrap();
    let guild_id = *ctx.guild_id().unwrap().as_u64() as i64;
    let feed_channel_id = *feed_channel.id().as_u64() as i64;

    let feed_role_id = match feed_role {
        Some(role) => *role.id.as_u64() as i64,
        None => 0
    };

    let tag_id = match grab_tag_id(heycafe_tag.clone(), heycafe_data["response_data"]["tags"].as_array()) {
        Ok(id) => id,
        Err(err) => return Err(err)
    };

    let feed_type = match api_feed_type {
        "account_info" => "user",
        _ => "cafe"
    };

    // Insert into DB and send msg
    sqlx::query!("INSERT INTO heycafe_feeds (guild_id, feed_type, channel_id, heycafe_id, last_post_id, mention_role_id, tag_id) VALUES (?, ?, ?, ?, 0, ?, ?)", guild_id, feed_type, feed_channel_id, heycafe_id, feed_role_id, tag_id)
        .execute(&ctx.data().database)
        .await
        .unwrap();

    let tag_addon = if tag_id != "none" {
        format!(" with the tag {}", heycafe_tag.unwrap())
    } else {
        String::new()
    };

    let msg = format!("Now listening to {alias} and posting in the channel {feed_channel}{tag_addon}!");
    ctx.say(msg).await?;
    Ok(())
}

/// Remove a feed that is being listened to!
#[poise::command(
    slash_command,
    required_permissions = "MANAGE_CHANNELS",
)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "Alias of the user or cafe."]
    #[max_length = 30] mut alias: String,

    #[description = "User/cafe tag being listened to."]
    #[rename = "tag"]
    #[max_length = 30] heycafe_tag: Option<String>
) -> Result<(), Error> {
    // Form URL and grab data
    let api_feed_type = match alias.chars().nth(0).unwrap() {
        '!' => {
            alias = alias.strip_prefix('!').unwrap().to_string();
            "cafe_info"
        },
        _ => {
            alias = alias.strip_prefix('@').unwrap_or(&alias).to_string();
            "account_info"
        }
    };
    
    let heycafe_data = match grab_feed_data(format!("https://endpoint.hey.cafe/api/{api_feed_type}?query={alias}&convert_numeric=tags"), &ctx.data().client).await {
        Ok(data) => data,
        Err(err) => return Err(err)
    };

    let tag_id = match grab_tag_id(heycafe_tag.clone(), heycafe_data["response_data"]["tags"].as_array()) {
        Ok(id) => id,
        Err(err) => return Err(err)
    };

    let heycafe_id = heycafe_data["response_data"]["id"].as_str().unwrap();
    let guild_id = *ctx.guild_id().unwrap().as_u64() as i64;

    // Check database then run query if found
    let db_check = sqlx::query!("SELECT COUNT(id) AS count FROM heycafe_feeds WHERE guild_id = ? AND heycafe_id = ? AND tag_id = ?", guild_id, heycafe_id, tag_id)
        .fetch_one(&ctx.data().database)
        .await
        .unwrap();

    if db_check.count == 0 {
        if heycafe_tag.is_some() {
            return Err(format!("No feed was found in the database with the alias \"{alias}\" and tag \"{}\"!", heycafe_tag.unwrap()).into());
        } else {
            return Err(format!("No feed was found in the database with the alias \"{alias}\"!").into());
        }
    }

    sqlx::query!("DELETE FROM heycafe_feeds WHERE guild_id = ? AND heycafe_id = ? AND tag_id = ?", guild_id, heycafe_id, tag_id)
        .execute(&ctx.data().database)
        .await
        .unwrap();

    let msg = if heycafe_tag.is_some() {
        format!("No longer listening to {alias} with the tag {}!", heycafe_tag.unwrap())
    } else {
        format!("No longer listening to {alias}!")
    };
    ctx.say(msg).await?;

    Ok(())
}

// Important funcs
// FUNCTION - Returns tag id from a given tag alias
fn grab_tag_id(tag_alias: Option<String>, tag_data: Option<&Vec<Value>>) -> Result<String, Error> {
    if tag_alias.is_none() {
        return Ok(String::from("none"));
    }

    if tag_data.is_none() {
        return Err("The cafe or user specified doesn't have tags!".into());
    }

    let tag_alias = tag_alias.unwrap();

    for tag in tag_data.unwrap() {
        if tag_alias == tag["name"].as_str().unwrap() {
            return Ok(tag["id"].as_str().unwrap().to_string());
        }
    }

    Err(format!("The tag \"{tag_alias}\" was not found!").into())
}