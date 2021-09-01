use diesel::prelude::*;
use rocket::{http::Status, serde::json::Json};

use crate::{
    models::{
        team::{validate_slug, NewTeam, Team},
        team_user::TeamUser,
        user::User,
    },
    DbConn,
};

#[post("/teams", data = "<team>")]
pub async fn create(user: User, team: Json<NewTeam>, conn: DbConn) -> Result<Json<Team>, Status> {
    if !validate_slug(&team.slug) {
        return Err(Status::UnprocessableEntity);
    }

    conn.run(move |c| {
        use crate::schema::team_users::dsl::*;
        use crate::schema::teams::dsl::*;

        let created_team = diesel::insert_into(teams)
            .values(team.0)
            .get_result::<Team>(c)
            .map_err(|_| Status::InternalServerError)?;

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
