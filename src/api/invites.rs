use diesel::{
	dsl::not,
	prelude::*,
	result::{
		DatabaseErrorKind::UniqueViolation,
		Error::{DatabaseError, NotFound},
	},
};
use rocket::{http::Status, serde::json::Json};

use db_models::{Invite, NewInvite, Team};

use crate::{auth::AuthUser, DbConn};

#[get("/teams/invite")]
pub async fn fetch(user: AuthUser, conn: DbConn) -> Result<Json<Vec<Team>>, Status> {
	conn.run(move |c| {
		use db_models::schema::invites::dsl::*;
		use db_models::schema::teams::dsl::teams;

		let user_invites = invites
			.filter(user_id.eq(user.id))
			.inner_join(teams)
			.load::<(Invite, Team)>(c)
			.map(|u| u.into_iter().map(|u| u.1).collect())
			.map_err(|_| Status::InternalServerError)?;

		Ok(Json(user_invites))
	})
	.await
}

#[post("/teams/invite", data = "<invite>")]
pub async fn create(
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
