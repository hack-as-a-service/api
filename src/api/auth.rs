use std::env;

use rocket::response::Redirect;

use crate::slack::{exchange_code, user_info};

#[get("/login")]
pub async fn login() -> Option<Redirect> {
    let client_id = env::var("SLACK_CLIENT_ID").ok()?;
    let redirect_uri = env::var("SLACK_REDIRECT_URI").ok()?;

    Some(Redirect::temporary(format!(
        "https://slack.com/openid/connect/authorize?response_type=code&scope=openid%20profile%20email&client_id={}&redirect_uri={}",
        client_id,
        redirect_uri,
    )))
}

#[get("/oauth/code?<code>")]
pub async fn code(code: &str) -> Result<String, String> {
    let client_id = env::var("SLACK_CLIENT_ID").map_err(|e| e.to_string())?;
    let client_secret = env::var("SLACK_CLIENT_SECRET").map_err(|e| e.to_string())?;
    let redirect_uri = env::var("SLACK_REDIRECT_URI").map_err(|e| e.to_string())?;

    let access_token = exchange_code(code, &client_id, &client_secret, &redirect_uri)
        .await
        .ok_or(String::from("sad"))?;
    let info = user_info(&access_token).await.expect("hi");

    Ok(info.name)
}
