use diesel::{prelude::*, result::Error::NotFound};
use rocket::{
    http::Status,
    request::{self, FromRequest, Outcome, Request},
};

use crate::{
    models::{token::Token, user::User},
    DbConn,
};

pub struct HaasUser(pub User);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for HaasUser {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let cookies = req.cookies();

        // Check Authorization header and cookies
        let token = req
            .headers()
            .get_one("Authorization")
            .and_then(|e| e.strip_prefix("Bearer "))
            .or_else(|| cookies.get("haas_token").map(|e| e.value()));

        // `?` only works on Option/Result types
        match DbConn::get_one(req.rocket()).await {
            Some(conn) => match token {
                Some(_t) => {
                    let t = _t.to_owned();

                    let user = conn
                        .run(|c| {
                            use crate::schema::tokens::dsl::*;
                            use crate::schema::users::dsl::*;

                            tokens
                                .filter(token.eq(t).and(expires_at.gt(diesel::dsl::now)))
                                .inner_join(users)
                                .first::<(Token, User)>(c)
                                .map(|e| e.1)
                        })
                        .await;

                    match user {
                        Ok(u) => Outcome::Success(Self(u)),
                        Err(e) => {
                            if e == NotFound {
                                Outcome::Failure((Status::Unauthorized, ()))
                            } else {
                                Outcome::Failure((Status::InternalServerError, ()))
                            }
                        }
                    }
                }
                None => request::Outcome::Failure((Status::Unauthorized, ())),
            },
            None => request::Outcome::Failure((Status::InternalServerError, ())),
        }
    }
}
