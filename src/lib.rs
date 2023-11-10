use serde_json::Value;

// Decode HTML from Café feeds
pub fn html_decode(mut content: String) -> String {
    content = content.replace("&#124;", "|");
    content = content.replace("&#92;", "\\\\");
    content = content.replace("&#34;", "\"");
    content = content.replace("&#39;", "'");
    content = content.replace("&#60;", "<");
    content = content.replace("&lt;", "<");
    content = content.replace("&#62;", ">");
    content = content.replace("&gt;", ">");
    content = content.replace("&#43;", "+");
    content = content.replace("&#96;", "`");

    return content;
}

// Escape some Discord markdown
pub fn escpae_markdown(mut content: String) -> String {
    content = content.replace("###", "");
    content = content.replace("##", "");
    content = content.replace("#", r#"\#"#);
    content = content.replace(r#"  "#, "\n");
    
    return content;
}

// Grab required role to do things from DB
struct HeyGuildSettings {
    id: i64,
    guild_id: i64,
    feed_settings_required_roleid: i64
}

pub async fn grab_required_role(db: &sqlx::SqlitePool, guild_id: i64) -> u64 {
    let required_role = sqlx::query_as!(HeyGuildSettings, "SELECT * FROM guild_settings WHERE guild_id = ?", guild_id)
        .fetch_one(db)
        .await
        .unwrap();

    return required_role.feed_settings_required_roleid as u64;
}

// Grab data from API
pub async fn grab_data(tag_id: &str, feed_type: &str, heycafe_id: &str, client: &reqwest::Client) -> Option<Value> {
    let tag_var = if tag_id != "none" {
        format!("&tag={}", tag_id)
    } else { String::new() };

    let api_feed_type = match feed_type {
        "user" => "account_conversations",
        "cafe" => "cafe_conversations",
        _ => ""
    };

    let api_feed_link = format!("https://endpoint.hey.cafe/api/{}?query={}&convert_numeric=conversations&count=1{}", api_feed_type, heycafe_id, tag_var);
    println!("[DEBUG] {api_feed_link}");

    let init_request = client.get(api_feed_link.clone())
        .send()
        .await;

    let init_request = match init_request {
        Ok(ok) => ok,
        Err(err) => {
            if err.is_timeout() {
                println!("Timeout");
            } else {
                println!("{}", err);
            }

            return None;
        }
    };

    
    let res = init_request
        .json::<serde_json::Value>()
        .await;

    match res {
        Ok(data) => {
            if data["response_data"]["conversations"].is_boolean() {
                println!("Conversation not found - API Link: {}", api_feed_link);
                return None;
            } else {
                return Some(data);
            }
        },
        Err(e) => {
            println!("JSON ERROR: {}", e);
            
            return None;
        }
    }
}

// Check API data for errors
pub fn has_error(data: &Value) -> bool {
    if !data["system_api_error"].is_boolean() {
        println!("API ERROR - Error: {}", data["system_api_error"].as_str().unwrap());
        return true;
    }

    return false;
}