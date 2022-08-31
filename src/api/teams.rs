use diesel::{
	dsl::not,
	prelude::*,
	result::{
		DatabaseErrorKind::{ForeignKeyViolation, UniqueViolation},
		Error::{DatabaseError, NotFound},
	},
};
use rocket::{http::Status, response::status::NoContent, serde::json::Json};

use db_models::{
	schema::invites, App, Invite, NewInvite, NewTeam, Team, TeamUser, UpdatedTeam, User,
};

use crate::{auth::AuthUser, utils::slug::validate_slug, DbConn};

/// Fetches a team by the `slug`, which can either be a Team.slug or a numeric Team.id
fn fetch_team(team_slug: String, user_id: i32, c: &diesel::PgConnection) -> QueryResult<Team> {
	use db_models::schema::team_users;
	use db_models::schema::teams::dsl::*;
	// Attempt to parse out a numeric ID
	let team = match team_slug.parse::<i32>() {
		Ok(i) => teams
			.filter(id.eq(i).and(team_users::user_id.eq(user_id)))
			.inner_join(team_users::table)
			.first::<(Team, TeamUser)>(c)
			.map(|x| x.0)?,
		Err(_) => teams
			.filter(slug.eq(team_slug).and(team_users::user_id.eq(user_id)))
			.inner_join(team_users::table)
			.first::<(Team, TeamUser)>(c)
			.map(|x| x.0)?,
	};

	Ok(team)
}

#[post("/teams", data = "<team>")]
pub async fn create(
	user: AuthUser,
	team: Json<NewTeam>,
	conn: DbConn,
) -> Result<Json<Team>, Status> {
	if !validate_slug(&team.slug) {
		return Err(Status::UnprocessableEntity);
	}

	conn.run(move |c| {
		use db_models::schema::team_users::dsl::*;
		use db_models::schema::teams::dsl::*;

		let created_team = diesel::insert_into(teams)
			.values(team.0)
			.get_result::<Team>(c)
			.map_err(|e| {
				if let DatabaseError(UniqueViolation, _) = e {
					Status::Conflict
				} else {
					Status::InternalServerError
				}
			})?;

		diesel::insert_into(team_users)
			.values(&TeamUser {
				team_id: created_team.id,
				user_id: user.id,
			})
			.execute(c)
			.map_err(|_| Status::InternalServerError)?;

		Ok(Json(created_team))
	})
	.await
}

#[post("/teams/invite", data = "<invite>")]
pub async fn invite(
	user: AuthUser,
	invite: Json<NewInvite>,
	conn: DbConn,
) -> Result<Json<Invite>, Status> {
	conn.run(move |c| {
		use db_models::schema::invites::dsl::*;

		let created_invite = diesel::insert_into(invites)
			.values(invite.0)
			.get_result::<Invite>(c)
			.map_err(|e| {
				if let DatabaseError(UniqueViolation, _) = e {
					Status::Conflict
				} else {
					Status::InternalServerError
				}
			})?;

		Ok(Json(created_invite))
	})
	.await
}

#[get("/teams/<team_slug>")]
pub async fn team(team_slug: String, user: AuthUser, conn: DbConn) -> Result<Json<Team>, Status> {
	conn.run(move |c| {
		let team = fetch_team(team_slug, user.id, c).map_err(|e| {
			if e == NotFound {
				Status::NotFound
			} else {
				Status::InternalServerError
			}
		})?;

		Ok(Json(team))
	})
	.await
}

#[get("/teams/<team_slug>/users")]
pub async fn users(
	team_slug: String,
	user: AuthUser,
	conn: DbConn,
) -> Result<Json<Vec<User>>, Status> {
	conn.run(move |c| {
		use db_models::schema::team_users::dsl::{team_id, team_users};
		use db_models::schema::users::dsl::users;

		// Fetch the team
		let team = fetch_team(team_slug, user.id, c).map_err(|e| {
			if e == NotFound {
				Status::NotFound
			} else {
				Status::InternalServerError
			}
		})?;

		// Fetch the team's users
		let loaded_users: Vec<User> = team_users
			.filter(team_id.eq(team.id))
			.inner_join(users)
			.load::<(TeamUser, User)>(c)
			.map(|u| u.into_iter().map(|u| u.1).collect())
			.map_err(|_| Status::InternalServerError)?;

		Ok(Json(loaded_users))
	})
	.await
}

#[get("/teams/<team_slug>/apps")]
pub async fn apps(
	team_slug: String,
	user: AuthUser,
	conn: DbConn,
) -> Result<Json<Vec<App>>, Status> {
	conn.run(move |c| {
		// Fetch the team
		let team = fetch_team(team_slug, user.id, c).map_err(|e| {
			if e == NotFound {
				Status::NotFound
			} else {
				Status::InternalServerError
			}
		})?;

		// Fetch the team's apps
		let loaded_apps = App::belonging_to(&team)
			.load::<App>(c)
			.map_err(|_| Status::InternalServerError)?;

		Ok(Json(loaded_apps))
	})
	.await
}

#[patch("/teams/<team_slug>", data = "<team>")]
pub async fn update(
	team_slug: String,
	user: AuthUser,
	conn: DbConn,
	team: Json<UpdatedTeam>,
) -> Result<Json<Team>, Status> {
	conn.run(move |c| {
		use db_models::schema::team_users::dsl::{team_users, user_id};
		use db_models::schema::teams::dsl::{id, personal, slug, teams};

		let (fetched_team, _) = teams
			.filter(
				slug.eq(&team_slug)
					.and(not(personal))
					.and(user_id.eq(user.id)),
			)
			.inner_join(team_users)
			.first::<(Team, TeamUser)>(c)
			.map_err(|e| {
				if e == NotFound {
					Status::NotFound
				} else {
					Status::InternalServerError
				}
			})?;

		let new_team = diesel::update(teams.filter(id.eq(fetched_team.id)))
			.set(&team.into_inner())
			.get_result::<Team>(c)
			.map_err(|e| {
				if let DatabaseError(UniqueViolation, _) = e {
					Status::Conflict
				} else {
					Status::InternalServerError
				}
			})?;

		Ok(Json(new_team))
	})
	.await
}

#[delete("/teams/<team_slug>")]
pub async fn delete(team_slug: String, user: AuthUser, conn: DbConn) -> Result<NoContent, Status> {
	conn.run(move |c| {
		use db_models::schema::team_users::dsl::{team_users, user_id};
		use db_models::schema::teams::dsl::{id, personal, slug, teams};

		let (fetched_team, _) = teams
			.filter(
				slug.eq(&team_slug)
					.and(not(personal))
					.and(user_id.eq(user.id)),
			)
			.inner_join(team_users)
			.first::<(Team, TeamUser)>(c)
			.map_err(|e| {
				if e == NotFound {
					Status::NotFound
				} else {
					Status::InternalServerError
				}
			})?;

		diesel::delete(teams.filter(id.eq(fetched_team.id)))
			.execute(c)
			.map_err(|e| {
				println!("{:?}", e);
				if let DatabaseError(ForeignKeyViolation, _) = e {
					Status::Conflict
				} else {
					Status::InternalServerError
				}
			})?;

		Ok(NoContent)
	})
	.await
}
