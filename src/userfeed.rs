use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use reqwest::{get, Client, header::USER_AGENT};
use serde_json::Value;
use botcafe::grab_required_role;
use serenity::{ChannelId, RoleId};

/// Add or remove user feeds to listen to.
#[poise::command(
    slash_command,
    subcommands("add", "delete"),
)]
pub async fn userfeed(_: Context<'_>) -> Result<(), Error> {
	Ok(())
}

/// Add a user feed
#[poise::command(slash_command)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "Username to listen to."]
    #[max_length = 50] username: String,
    #[description = "Channel to post to."] discord_channel: serenity::Channel,
    #[description = "Role to ping for new posts."] mention_role: Option<serenity::Role>,
    #[description = "Specify tag to only grab posts with a given tag."]
    #[max_length = 50] tag_name: Option<String>
) -> Result<(), Error> {
    let guild_id = *ctx.guild_id().unwrap().as_u64() as i64;

    // Check user's role
    let req_role = grab_required_role(&ctx.data().database, guild_id).await;
    if req_role == 0 {
        let msg = format!("{}, you must set a required role to use these commands with `/botcafe feed_role`!", ctx.author());
        ctx.say(msg).await?;
        return Ok(());
    }
    let req_role = RoleId(req_role);

    if let Some(member) = ctx.author_member().await {
        if !member.roles.contains(&req_role) {
            let role_name = req_role.to_role_cached(&ctx).unwrap().name;
            let msg = format!("{}, you must have the {} role!", ctx.author(), role_name);
            ctx.say(msg).await?;
            return Ok(());
        }
    }

    // Validate username
    let api_link = format!("https://endpoint.hey.cafe/api/account_info?query={username}&convert_numeric=tags");
    let api_user_data = ctx.data().client.get(api_link)
        .header(USER_AGENT, "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko)")
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    //println!("{:#?}", api_user_data);
    if !api_user_data["system_api_error"].is_boolean() {
        let msg = format!("{}, user *@{username}* not found!", ctx.author());
        ctx.say(msg).await?;
        return Ok(());
    }
    let heycafe_id = api_user_data["response_data"]["id"].as_str().unwrap().to_string();

    // If role set, add it
    let mut role_id: i64 = 0;
    if !mention_role.is_none() {
        role_id = *mention_role.as_ref().unwrap().id.as_u64() as i64;
    }

    // If tag specified
    let mut tag_id = String::from("none");
    if !tag_name.is_none() {
        if api_user_data["response_data"]["tags"].is_boolean() {
            let msg = format!("{}, user *@{}* does not have any tags!", ctx.author(), username);
            ctx.say(msg).await?;
            return Ok(());
        }

        for i in 0..=5 {
            if api_user_data["response_data"]["tags"][i]["name"].as_str().unwrap_or("completed tag search") == "completed tag search" {
                let msg = format!("{}, no tag with the name *{}* was found for user *@{}*!", ctx.author(), tag_name.unwrap(), username);
                ctx.say(msg).await?;
                return Ok(());
            }

            if tag_name.as_ref().unwrap() == api_user_data["response_data"]["tags"][i]["name"].as_str().unwrap() {
                tag_id = api_user_data["response_data"]["tags"][i]["id"].as_str().unwrap().to_string();
                break;
            }
        }
    }

    let channel_id = *discord_channel.id().as_u64() as i64;

    sqlx::query!("INSERT INTO heycafe_feeds (guild_id, feed_type, channel_id, heycafe_id, last_post_id, mention_role_id, tag_id) VALUES (?, \"user\", ?, ?, 0, ?, ?)", guild_id, channel_id, heycafe_id, role_id, tag_id)
        .execute(&ctx.data().database)
        .await
        .unwrap();

    let add_tags = {
        if tag_id.as_str() != "none" {
            format!(" with tag {}", tag_name.as_ref().unwrap())
        } else {
            String::new()
        }
    };
    let msg = format!("{}, now listening to user *@{}* in {}{}!", ctx.author(), username, discord_channel, add_tags);
    ctx.say(msg).await?;
    println!("[LOG] COMMAND: /userfeed add {} {} {:?} {:?} - Guild: {}({})", username, discord_channel, mention_role, tag_name, ctx.guild().unwrap().name, guild_id);

    Ok (())
}

/// Delete a user feed
#[poise::command(slash_command)]
pub async fn delete(
    ctx: Context<'_>,
    #[description = "Username to stop listening to."] username: String,
    #[description = "Channel the feed is posted to."] discord_channel: serenity::Channel,
) -> Result<(), Error> {
    let guild_id = *ctx.guild_id().unwrap().as_u64() as i64;

    // Check user's role
    let req_role = grab_required_role(&ctx.data().database, guild_id).await;
    if req_role == 0 {
        let msg = format!("{}, you must set a required role to use these commands with `/botcafe feed_role`!", ctx.author());
        ctx.say(msg).await?;
        return Ok(());
    }
    let req_role = RoleId(req_role);

    if let Some(member) = ctx.author_member().await {
        if !member.roles.contains(&req_role) {
            let role_name = req_role.to_role_cached(&ctx).unwrap().name;
            let msg = format!("{}, you must have the {} role!", ctx.author(), role_name);
            ctx.say(msg).await?;
            return Ok(());
        }
    }

    // Validate username in API
    let api_link = format!("https://endpoint.hey.cafe/api/account_info?query={username}&convert_numeric=tags");
    let api_user_data = ctx.data().client.get(api_link)
        .header(USER_AGENT, "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko)")
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    //println!("{:#?}", api_user_data);
    if !api_user_data["system_api_error"].is_boolean() {
        let msg = format!("{}, user *@{username}* not found in Hey.Café!", ctx.author());
        ctx.say(msg).await?;
        return Ok(());
    }
    let heycafe_id = api_user_data["response_data"]["id"].as_str().unwrap().to_string();
    let channel_id = *discord_channel.id().as_u64() as i64;

    // Validate username in database
    let db_check = sqlx::query!("SELECT COUNT(id) AS count FROM heycafe_feeds WHERE guild_id = ? AND heycafe_id = ? AND channel_id = ?", guild_id, heycafe_id, channel_id)
        .fetch_one(&ctx.data().database)
        .await
        .unwrap();

    if db_check.count == 0 {
        let msg = format!("{}, user *@{username}* not found in the bot database!", ctx.author());
        ctx.say(msg).await?;
        return Ok(());
    }

    sqlx::query!("DELETE FROM heycafe_feeds WHERE guild_id = ? AND heycafe_id = ? AND channel_id = ?", guild_id, heycafe_id, channel_id)
        .execute(&ctx.data().database)
        .await
        .unwrap();
    
    let msg = format!("{}, no longer listening to user *@{}*!", ctx.author(), username);
    ctx.say(msg).await?;
    println!("[LOG] COMMAND: /userfeed delete {} {} - Guild: {}({})", username, discord_channel, ctx.guild().unwrap().name, guild_id);

    Ok(())
}

