use diesel::{
	dsl::not,
	prelude::*,
	result::{
		DatabaseErrorKind::UniqueViolation,
		Error::{DatabaseError, NotFound},
	},
};
use rocket::{http::Status, response::status::NoContent, serde::json::Json};

use db_models::{Invite, NewInvite, Team};

use crate::{auth::AuthUser, DbConn};

#[get("/teams/invite")]
pub async fn get(user: AuthUser, conn: DbConn) -> Result<Json<Vec<Team>>, Status> {
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
			.values(&Invite {
				team_id: invite.0.team_id,
				user_id: user.id,
			})
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

// Used to accept team invites
#[delete("/teams/invite/<id>/accept")]
pub async fn accept(id: String, user: AuthUser, conn: DbConn) -> Result<NoContent, Status> {
	use db_models::schema::invites::dsl::{invites, team_id, user_id};
	use db_models::schema::team_users::dsl::*;

	conn.run(move |c| {
		diesel::insert_into(team_users)
			.values(&TeamUser {
				team_id: id,
				user_id: user.id,
			})
			.execute(c)
			.map_err(|_| Status::InternalServerError)?;

		diesel::delete(invites.filter(user_id).eq(user.id).and(team_id.eq(id)))
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

// Used to revoke team invites
#[delete("/teams/invite/<id>/delete")]
pub async fn delete(id: String, user: AuthUser, conn: DbConn) -> Result<NoContent, Status> {
	use db_models::schema::invites::dsl::{invites, team_id, user_id};
	conn.run(move |c| {
		let result = diesel::delete(invites.filter(user_id).eq(user.id).and(team_id.eq(id)))
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
