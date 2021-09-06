use diesel::{
    prelude::*,
    result::{
        DatabaseErrorKind::UniqueViolation,
        Error::{DatabaseError, NotFound},
    },
};
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

#[get("/teams/<team_slug>")]
pub async fn team(team_slug: String, user: User, conn: DbConn) -> Result<Json<Team>, Status> {
    conn.run(move |c| {
        use crate::schema::team_users::dsl::{team_users, user_id};
        use crate::schema::teams::dsl::{slug, teams};

        let team = teams
            .filter(slug.eq(team_slug).and(user_id.eq(user.id)))
            .inner_join(team_users)
            .first::<(Team, TeamUser)>(c)
            .map_err(|e| {
                if e == NotFound {
                    Status::NotFound
                } else {
                    Status::InternalServerError
                }
            })
            .map(|x| x.0)?;

        Ok(Json(team))
    })
    .await
}

#[get("/teams/<team_slug>/users")]
pub async fn users(team_slug: String, user: User, conn: DbConn) -> Result<Json<Vec<User>>, Status> {
    conn.run(move |c| {
        use crate::schema::team_users::dsl::{team_users, user_id};
        use crate::schema::teams::dsl::{slug, teams};
        use crate::schema::users::dsl::users;

        // Fetch the team
        let team = teams
            .filter(slug.eq(&team_slug).and(user_id.eq(user.id)))
            .inner_join(team_users)
            .first::<(Team, TeamUser)>(c)
            .map(|x| x.0)
            .map_err(|e| {
                if e == NotFound {
                    Status::NotFound
                } else {
                    Status::InternalServerError
                }
            })?;

        // Fetch the team's users
        let loaded_users: Vec<User> = teams
            .find(team.id)
            .inner_join(team_users.inner_join(users))
            .load::<(Team, (TeamUser, User))>(c)
            .map_err(|_| Status::InternalServerError)
            .map(|u| u.into_iter().map(|u| u.1 .1).collect())?;

        Ok(Json(loaded_users))
    })
    .await
}
