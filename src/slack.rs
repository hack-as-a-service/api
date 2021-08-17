use serde::Deserialize;

/// Exchanges an authorization code for a Slack access token
pub async fn exchange_code(
    code: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
) -> Option<String> {
    let client = reqwest::Client::new();

    #[derive(Deserialize)]
    struct AccessTokenResponse {
        access_token: Option<String>,
    }

    let resp = client
        .post("https://slack.com/api/openid.connect.token")
        .form(&[
            ("code", code),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("redirect_uri", &redirect_uri),
        ])
        .send()
        .await
        .ok()?
        .json::<AccessTokenResponse>()
        .await
        .ok()?;

    resp.access_token
}

#[derive(Deserialize)]
pub struct UserInfo {
    pub name: String,
    pub picture: String,
}

pub async fn user_info(token: &str) -> Result<UserInfo, ()> {
    let client = reqwest::Client::new();

    let resp = client
        .post("https://slack.com/api/openid.connect.userInfo")
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| ())?
        .json::<UserInfo>()
        .await
        .map_err(|_| ())?;

    Ok(resp)
}
