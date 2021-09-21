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
        app::{App, NewApp},
        team::Team,
        team_user::TeamUser,
        user::User,
    },
    utils::slug::validate_slug,
    DbConn,
};

#[get("/apps/<app_slug>")]
pub async fn app(app_slug: String, user: User, conn: DbConn) -> Result<Json<App>, Status> {
    conn.run(move |c| {
        use crate::schema::apps::dsl::{apps, slug};
        use crate::schema::team_users::dsl::{team_users, user_id};
        use crate::schema::teams::dsl::teams;

        let loaded_app = apps
            .inner_join(teams.inner_join(team_users))
            .filter(user_id.eq(user.id).and(slug.eq(app_slug)))
            .first::<(App, (Team, TeamUser))>(c)
            .map_err(|e| {
                if e == NotFound {
                    Status::NotFound
                } else {
                    Status::InternalServerError
                }
            })?;

        Ok(Json(loaded_app.0))
    })
    .await
}

#[post("/teams/<team_slug>/apps", data = "<app>")]
pub async fn create(
    user: User,
    team_slug: String,
    app: Json<NewApp>,
    conn: DbConn,
) -> Result<Json<App>, Status> {
    if !validate_slug(&app.slug) {
        return Err(Status::UnprocessableEntity);
    }

    conn.run(move |c| {
        use crate::schema::apps::dsl::apps;
        use crate::schema::team_users::dsl::{team_users, user_id};
        use crate::schema::teams::dsl::{slug, teams};

        let (team, _) = teams
            .filter(slug.eq(team_slug).and(user_id.eq(user.id)))
            .inner_join(team_users)
            .first::<(Team, TeamUser)>(c)
            .map_err(|e| {
                if e == NotFound {
                    Status::NotFound
                } else {
                    Status::InternalServerError
                }
            })?;

        let app = diesel::insert_into(apps)
            .values(NewApp {
                team_id: team.id,
                ..app.0
            })
            .get_result::<App>(c)
            .map_err(|e| {
                if let DatabaseError(UniqueViolation, _) = e {
                    Status::Conflict
                } else {
                    Status::InternalServerError
                }
            })?;

        Ok(Json(app))
    })
    .await
}
