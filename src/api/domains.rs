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
        app::App,
        domain::{validate_domain, Domain, NewDomain},
        team::Team,
        team_user::TeamUser,
        user::User,
    },
    DbConn,
};

#[post("/apps/<app_slug>/domains", data = "<domain>")]
pub async fn create(
    app_slug: String,
    domain: Json<NewDomain>,
    user: User,
    conn: DbConn,
) -> Result<Json<Domain>, Status> {
    if !validate_domain(&domain.domain) {
        return Err(Status::UnprocessableEntity);
    }

    conn.run(move |c| {
        use crate::schema::apps::dsl::{apps, slug};
        use crate::schema::domains::dsl::domains;
        use crate::schema::team_users::dsl::{team_users, user_id};
        use crate::schema::teams::dsl::teams;

        let (app, _) = apps
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

        let created_domain = diesel::insert_into(domains)
            .values(&NewDomain {
                app_id: app.id,
                verified: false,
                ..domain.0
            })
            .get_result::<Domain>(c)
            .map_err(|e| {
                if let DatabaseError(UniqueViolation, _) = e {
                    Status::Conflict
                } else {
                    Status::InternalServerError
                }
            })?;

        Ok(Json(created_domain))
    })
    .await
}
