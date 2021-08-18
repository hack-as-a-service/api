use diesel::{prelude::*, result::Error::NotFound};
use rocket::{
    http::Status,
    request::{self, FromRequest, Outcome, Request},
};

use crate::{
    models::{token::Token, user::User},
    DbConn,
};

pub struct HaaSUser(pub User);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for HaaSUser {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let cookies = req.cookies();

        // `?` only works on Option/Result types
        match DbConn::get_one(req.rocket()).await {
            Some(conn) => match cookies.get("haas_token") {
                Some(_t) => {
                    let t = _t.value().to_owned();

                    let user = conn
                        .run(|c| {
                            use crate::schema::tokens::dsl::*;
                            use crate::schema::users::dsl::*;

                            tokens
                                .filter(token.eq(t))
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
