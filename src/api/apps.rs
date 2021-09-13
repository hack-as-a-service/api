use diesel::{prelude::*, result::Error::NotFound};
use rocket::{http::Status, serde::json::Json};

use crate::{
    models::{app::App, team::Team, team_user::TeamUser, user::User},
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
