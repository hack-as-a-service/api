use diesel::prelude::*;
use rocket::{http::Status, serde::json::Json};

use db_models::{
    Team,
    TeamUser,
    User,
};

use crate::{
    auth::AuthUser,
    DbConn,
};

#[get("/users/me")]
pub fn me(user: AuthUser) -> Json<User> {
    Json(user.into_inner())
}

#[get("/users/me/teams")]
pub async fn teams(user: AuthUser, conn: DbConn) -> Result<Json<Vec<Team>>, Status> {
    conn.run(move |c| {
        use db_models::schema::team_users::dsl::*;
        use db_models::schema::teams::dsl::*;

        let loaded_teams = team_users
            .filter(user_id.eq(user.id))
            .inner_join(teams)
            .load::<(TeamUser, Team)>(c)
            .map_err(|_| Status::InternalServerError)?;

        Ok(Json(loaded_teams.into_iter().map(|t| t.1).collect()))
    })
    .await
}
