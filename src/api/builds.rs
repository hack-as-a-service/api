use diesel::{prelude::*, result::Error::NotFound};
use rocket::{http::Status, serde::json::Json};

use db_models::{App, Build, Team, TeamUser};

use crate::{auth::AuthUser, DbConn};

#[get("/builds/<build_id>")]
pub async fn build(build_id: i32, user: AuthUser, conn: DbConn) -> Result<Json<Build>, Status> {
	conn.run(move |c| {
		use db_models::schema::apps::dsl::apps;
		use db_models::schema::builds::dsl::{builds, id};
		use db_models::schema::team_users::dsl::{team_users, user_id};
		use db_models::schema::teams::dsl::teams;

		let build = builds
			.inner_join(apps.inner_join(teams.inner_join(team_users)))
			.filter(user_id.eq(user.id).and(id.eq(build_id)))
			.first::<(Build, (App, (Team, TeamUser)))>(c)
			.map_err(|e| {
				if e == NotFound {
					Status::NotFound
				} else {
					Status::InternalServerError
				}
			})?;

		Ok(Json(build.0))
	})
	.await
}
