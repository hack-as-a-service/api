use diesel::{
    connection::Connection,
    prelude::*,
    result::{
        DatabaseErrorKind::UniqueViolation,
        Error::{self, DatabaseError, NotFound},
    },
};
use rocket::{http::Status, serde::json::Json};

use db_models::{App, Domain, NewApp, NewDomain, Team, TeamUser};

use crate::{auth::AuthUser, utils::slug::validate_slug, DbConn};

#[get("/apps/<app_slug>")]
pub async fn app(app_slug: String, user: AuthUser, conn: DbConn) -> Result<Json<App>, Status> {
    conn.run(move |c| {
        use db_models::schema::apps::dsl::{apps, slug};
        use db_models::schema::team_users::dsl::{team_users, user_id};
        use db_models::schema::teams::dsl::teams;

        let app = apps
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

        Ok(Json(app.0))
    })
    .await
}

#[post("/teams/<team_slug>/apps", data = "<app>")]
pub async fn create(
    user: AuthUser,
    team_slug: String,
    app: Json<NewApp>,
    conn: DbConn,
) -> Result<Json<App>, Status> {
    if !validate_slug(&app.slug) {
        return Err(Status::UnprocessableEntity);
    }

    conn.run(move |c| {
        use db_models::schema::apps::dsl::apps;
        use db_models::schema::domains::dsl::domains;
        use db_models::schema::team_users::dsl::{team_users, user_id};
        use db_models::schema::teams::dsl::{slug, teams};

        // Fetch the team
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

        let app = Connection::transaction::<App, Error, _>(c, || {
            // Create the app
            let app = diesel::insert_into(apps)
                .values(NewApp {
                    team_id: team.id,
                    ..app.0
                })
                .get_result::<App>(c)?;

            // Create the app's initial domain
            diesel::insert_into(domains)
                .values(NewDomain {
                    verified: true,
                    domain: format!("{}.hackclub.app", app.slug),
                    app_id: app.id,
                })
                .execute(c)?;

            Ok(app)
        })
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

#[get("/apps/<app_slug>/domains")]
pub async fn domains(
    app_slug: String,
    user: AuthUser,
    conn: DbConn,
) -> Result<Json<Vec<Domain>>, Status> {
    conn.run(move |c| {
        use db_models::schema::apps::dsl::{apps, slug};
        use db_models::schema::team_users::dsl::{team_users, user_id};
        use db_models::schema::teams::dsl::teams;

        let app = apps
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

        let domains = Domain::belonging_to(&app.0)
            .load::<Domain>(c)
            .map_err(|_| Status::InternalServerError)?;

        Ok(Json(domains))
    })
    .await
}
