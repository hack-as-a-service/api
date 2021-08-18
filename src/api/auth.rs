use std::env;

use diesel::prelude::*;
use rocket::{
    http::{Cookie, CookieJar, SameSite, Status},
    response::Redirect,
};

use time::Duration;

use crate::{
    models::{
        token::{generate_token, NewToken, Token},
        user::{NewUser, User},
    },
    slack::{exchange_code, user_info},
    DbConn,
};

#[get("/login")]
pub async fn login() -> Result<Redirect, Status> {
    let client_id = env::var("SLACK_CLIENT_ID").map_err(|_| Status::InternalServerError)?;
    let redirect_uri = env::var("SLACK_REDIRECT_URI").map_err(|_| Status::InternalServerError)?;

    Ok(Redirect::temporary(format!(
        "https://slack.com/openid/connect/authorize?response_type=code&scope=openid%20profile%20email&client_id={}&redirect_uri={}",
        client_id,
        redirect_uri,
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

#[get("/oauth/code?<code>")]
pub async fn code(conn: DbConn, code: &str, cookies: &CookieJar<'_>) -> Result<Redirect, Status> {
    let client_id = env::var("SLACK_CLIENT_ID").map_err(|_| Status::InternalServerError)?;
    let client_secret = env::var("SLACK_CLIENT_SECRET").map_err(|_| Status::InternalServerError)?;
    let redirect_uri = env::var("SLACK_REDIRECT_URI").map_err(|_| Status::InternalServerError)?;

    let access_token = exchange_code(code, &client_id, &client_secret, &redirect_uri)
        .await
        .ok_or(Status::InternalServerError)?;
    let info = user_info(&access_token)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let token = conn
        .run(|c| -> Result<Token, ()> {
            use crate::schema::tokens::dsl::*;
            use crate::schema::users::dsl::*;

            let user = users
                .filter(slack_user_id.eq(&info.user_id))
                .first::<User>(c)
                .or_else(|_| {
                    diesel::insert_into(users)
                        .values(&NewUser {
                            slack_user_id: info.user_id,
                            name: Some(info.name),
                            avatar: Some(info.picture),
                        })
                        .get_result::<User>(c)
                })
                .map_err(|_| ())?;

            let created_token = diesel::insert_into(tokens)
                .values(&NewToken {
                    token: generate_token(),
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
            .http_only(true)
            .max_age(Duration::seconds(2592000))
            .same_site(SameSite::None)
            .secure(true)
            .finish(),
    );

    // user.name.ok_or(Status::InternalServerError)
    Ok(Redirect::temporary("/"))
}