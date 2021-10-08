use std::env;

use base64::{decode, encode};
use diesel::prelude::*;
use form_urlencoded::Serializer;
use rocket::{
    http::{Cookie, CookieJar, SameSite, Status},
    response::Redirect,
};

use serde::{Deserialize, Serialize};
use time::Duration;

use db_models::{NewTeam, NewToken, NewUser, Team, TeamUser, Token, User};

use crate::{
    slack::{exchange_code, user_info},
    utils::{slug::into_slug, token::generate_token},
    DbConn,
};

#[derive(Serialize, Deserialize, Debug, Default)]
struct SlackOAuthState {
    #[serde(rename = "r")]
    return_to: Option<String>,
}

impl SlackOAuthState {
    fn to_state(&self) -> String {
        // .unwrap is safe here because we know SlackOAuthState is fully serializable
        let json = serde_json::to_string(self).unwrap();

        encode(json)
    }

    fn from_state(state: &str) -> Self {
        match decode(state) {
            Ok(x) => match serde_json::from_slice::<SlackOAuthState>(&x) {
                Ok(y) => y,
                Err(_) => Self::default(),
            },
            Err(_) => Self::default(),
        }
    }
}

#[get("/login?<return_to>")]
pub async fn login(return_to: Option<String>) -> Result<Redirect, Status> {
    let client_id = env::var("SLACK_CLIENT_ID").map_err(|_| Status::InternalServerError)?;
    let redirect_uri = env::var("SLACK_REDIRECT_URI").map_err(|_| Status::InternalServerError)?;

    let mut serializer = Serializer::new(String::from(""));

    serializer.append_pair("response_type", "code");
    serializer.append_pair("scope", "openid profile email");
    serializer.append_pair("client_id", &client_id);
    serializer.append_pair("redirect_uri", &redirect_uri);
    serializer.append_pair("state", &SlackOAuthState { return_to }.to_state());

    Ok(Redirect::temporary(format!(
        "https://slack.com/openid/connect/authorize?{}",
        serializer.finish()
    )))
}

#[get("/logout")]
pub async fn logout(conn: DbConn, cookies: &CookieJar<'_>) -> Redirect {
    if let Some(cookie) = cookies.get("haas_token") {
        let value = cookie.value().to_owned();

        conn.run(|c| {
            use crate::schema::tokens::dsl::*;

            diesel::delete(tokens.filter(token.eq(value)))
                .execute(c)
                // Ignore error
                .ok();
        })
        .await;

        cookies.remove(Cookie::named("haas_token"));
    }

    Redirect::temporary("/")
}

#[get("/oauth/code?<code>&<state>")]
pub async fn code(
    conn: DbConn,
    code: &str,
    state: Option<&str>,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, Status> {
    let client_id = env::var("SLACK_CLIENT_ID").map_err(|_| Status::InternalServerError)?;
    let client_secret = env::var("SLACK_CLIENT_SECRET").map_err(|_| Status::InternalServerError)?;
    let redirect_uri = env::var("SLACK_REDIRECT_URI").map_err(|_| Status::InternalServerError)?;

    let state = state.map_or(SlackOAuthState::default(), SlackOAuthState::from_state);

    let access_token = exchange_code(code, &client_id, &client_secret, &redirect_uri)
        .await
        .ok_or(Status::InternalServerError)?;
    let info = user_info(&access_token)
        .await
        .map_err(|_| Status::InternalServerError)?;

    // let whitelisted = if env::var("HAAS_PRODUCTION").is_err() {
    //     true
    // } else {
    //     let info = info.clone();

    //     conn.run(|c| {
    //         use crate::schema::whitelist::dsl::*;

    //         whitelist
    //             .find(info.user_id)
    //             .first::<WhitelistEntry>(c)
    //             .is_ok()
    //     })
    //     .await
    // };

    // if !whitelisted {
    //     return Err(Status::Forbidden);
    // }

    let token = conn
        .run(|c| -> Result<Token, ()> {
            use db_models::schema::team_users::dsl::*;
            use db_models::schema::teams::dsl::*;
            use db_models::schema::tokens::dsl::*;
            use db_models::schema::users::dsl::*;

            let user = users
                .filter(slack_user_id.eq(&info.user_id))
                .first::<User>(c)
                .or_else(|_| -> QueryResult<User> {
                    // Create user
                    let user = diesel::insert_into(users)
                        .values(&NewUser {
                            slack_user_id: info.user_id,
                            name: info.name.clone(),
                            avatar: Some(info.picture),
                        })
                        .get_result::<User>(c)?;

                    // Create the user's personal team
                    let team = diesel::insert_into(teams)
                        .values(&NewTeam {
                            name: Some(format!("{}'s team", info.name)),
                            slug: into_slug(&info.name, true),
                            avatar: None,
                            personal: true,
                        })
                        .get_result::<Team>(c)?;

                    // Add the user to their personal team
                    diesel::insert_into(team_users)
                        .values(&TeamUser {
                            team_id: team.id,
                            user_id: user.id,
                        })
                        .execute(c)?;

                    Ok(user)
                })
                .map_err(|_| ())?;

            let created_token = diesel::insert_into(tokens)
                .values(&NewToken {
                    token: &generate_token(),
                    user_id: user.id,
                })
                .get_result::<Token>(c)
                .map_err(|_| ())?;

            Ok(created_token)
        })
        .await
        .map_err(|_| Status::InternalServerError)?;

    cookies.add(
        Cookie::build("haas_token", token.token.clone())
            .path("/")
            .http_only(true)
            .max_age(Duration::seconds(2592000))
            .same_site(SameSite::Strict)
            .secure(true)
            .finish(),
    );

    Ok(Redirect::temporary(
        state.return_to.unwrap_or(String::from("/")),
    ))
}
