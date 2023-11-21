use serde_json::Value;
type Error = Box<dyn std::error::Error + Send + Sync>;

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

    content
}

// Escape some Discord markdown
pub fn escpae_markdown(mut content: String) -> String {
    content = content.replace("###", "");
    content = content.replace("##", "");
    content = content.replace('#', r#"\#"#);
    content = content.replace(r#"  "#, "\n");
    
    content
}

// FUNCTION - Returns API data from Hey.Cafe as a Result
pub async fn grab_feed_data(url: String, client: &reqwest::Client) -> Result<Value, Error> {
    let init_request = client.get(url.clone())
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

            return Err("There was an error requesting information!".into());
        }
    };
    
    let heycafe_data = init_request
        .json::<serde_json::Value>()
        .await;

    match heycafe_data {
        Ok(data) => {
            if data["system_api_error"].is_boolean() {
                Ok(data)
            } else {
                println!("Feed not found - API Link: {}", url);
                Err("No information was found!".into())
            }
        },
        Err(e) => {
            println!("JSON ERROR: {}", e);
            Err("There was an error handling information!".into())
        }
    }
}

// Check API data for errors
pub fn has_error(data: &Value) -> bool {
    if !data["system_api_error"].is_boolean() {
        println!("API ERROR - Error: {}", data["system_api_error"].as_str().unwrap());
        return true;
    }

    false
}