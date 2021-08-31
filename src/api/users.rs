use diesel::prelude::*;
use rocket::{http::Status, serde::json::Json};

use crate::{
    models::{team::Team, team_user::TeamUser, user::User},
    DbConn,
};

#[get("/users/me")]
pub fn me(user: User) -> Json<User> {
    Json(user)
}

#[get("/users/me/teams")]
pub async fn teams(user: User, conn: DbConn) -> Result<Json<Vec<Team>>, Status> {
    conn.run(move |c| {
        use crate::schema::team_users::dsl::*;
        use crate::schema::teams::dsl::*;

        let loaded_teams = team_users
            .filter(user_id.eq(user.id))
            .inner_join(teams)
            .load::<(TeamUser, Team)>(c)
            .map_err(|_| Status::InternalServerError)?;

        Ok(Json(loaded_teams.into_iter().map(|t| t.1).collect()))
    })
    .await
}
