#[macro_use]
extern crate diesel;

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate lazy_static;

use diesel::prelude::*;
use dotenv::dotenv;
use rocket::fs::NamedFile;
use rocket_sync_db_pools::database;

mod api;
mod models;
mod schema;
mod slack;

#[database("db")]
pub struct DbConn(PgConnection);

#[get("/openapi.yaml")]
async fn openapi() -> NamedFile {
    NamedFile::open("openapi/openapi.yaml").await.unwrap()
}

#[launch]
fn rocket() -> _ {
    dotenv().ok();

    rocket::build()
        .mount(
            "/api",
            routes![
                openapi,
                api::auth::login,
                api::auth::logout,
                api::auth::code,
                api::teams::apps,
                api::teams::create,
                api::teams::team,
                api::teams::users,
                api::users::me,
                api::users::teams
            ],
        )
        .attach(DbConn::fairing())
}
