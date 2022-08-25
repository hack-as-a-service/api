use std::env;

use db_models::{NewTeam, NewToken, Team, TeamUser, Token, User};
use rocket::{
	http::{Cookie, CookieJar, SameSite, Status},
	response::Redirect,
};

use diesel::prelude::*;
use time::Duration;

use nanoid::nanoid;

use crate::{
	utils::{slug::into_slug, token::generate_token},
	DbConn,
};

#[get("/dev/login")]
pub async fn login(conn: DbConn, cookies: &CookieJar<'_>) -> Result<Redirect, Status> {
	if env::var("HAAS_PRODUCTION").is_ok() {
		return Err(Status::NotFound);
	}

	let token = conn
		.run(|c| {
			use db_models::schema::team_users::dsl::team_users;
			use db_models::schema::teams::dsl::teams;
			use db_models::schema::tokens::dsl::tokens;
			use db_models::schema::users::dsl::*;

			let user = users
				.filter(id.eq(1))
				.first::<User>(c)
				.or_else(|_| -> QueryResult<User> {
					// Create user
					let user = diesel::insert_into(users)
						.values((
							id.eq(1),
							name.eq(String::from("Test user")),
							slack_user_id.eq(String::from("U123456789")),
						))
						.get_result::<User>(c)?;

					// Create the user's personal team
					let team = diesel::insert_into(teams)
						.values(&NewTeam {
							name: Some(format!("{}'s team", &user.name)),
							slug: into_slug(&user.name, true),
							avatar: None,
							personal: true,
              invite: nanoid!(7),
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
				.map_err(|_| Status::InternalServerError)?;

			let created_token = diesel::insert_into(tokens)
				.values(&NewToken {
					token: &generate_token(),
					user_id: user.id,
				})
				.get_result::<Token>(c)
				.map_err(|_| Status::InternalServerError)?;

			Ok(created_token)
		})
		.await?;

	cookies.add(
		Cookie::build("haas_token", token.token)
			.http_only(true)
			.max_age(Duration::seconds(2592000))
			.same_site(SameSite::Strict)
			.secure(false)
			.finish(),
	);

	Ok(Redirect::temporary("/"))
}
