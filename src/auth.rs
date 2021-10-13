use db_models::{Token, User};
use diesel::{prelude::*, result::Error::NotFound};
use rocket::{
	http::Status,
	request::{self, FromRequest, Outcome, Request},
};

use crate::DbConn;

#[repr(transparent)]
pub struct AuthUser(User);

// There is no DerefMut impl since an AuthUser can't be modified, it just represents the current
// authentication state.
impl std::ops::Deref for AuthUser {
	type Target = User;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl AuthUser {
	fn new(u: User) -> Self {
		Self(u)
	}

	pub fn into_inner(self) -> User {
		self.0
	}
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthUser {
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
							use db_models::schema::tokens::dsl::*;
							use db_models::schema::users::dsl::*;

							tokens
								.filter(token.eq(t).and(expires_at.gt(diesel::dsl::now)))
								.inner_join(users)
								.first::<(Token, User)>(c)
								.map(|e| e.1)
						})
						.await;

					match user {
						Ok(u) => Outcome::Success(AuthUser::new(u)),
						Err(e) => {
							if e == NotFound {
								Outcome::Failure((Status::Unauthorized, ()))
							} else {
								Outcome::Failure((Status::InternalServerError, ()))
							}
						}
					}
				}
				None => Outcome::Failure((Status::Unauthorized, ())),
			},
			None => Outcome::Failure((Status::InternalServerError, ())),
		}
	}
}
