#[macro_use]
extern crate diesel;

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate lazy_static;

use diesel::prelude::*;
use dotenv::dotenv;
use rocket_sync_db_pools::database;

mod api;
mod auth;
mod schema;
mod slack;
mod utils;

#[database("db")]
pub struct DbConn(PgConnection);

#[get("/openapi.yaml")]
async fn openapi() -> &'static str {
    include_str!("../openapi/openapi.yaml")
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
                api::apps::app,
                api::apps::create,
                api::apps::domains,
                api::dev::login,
                api::domains::create,
                api::teams::apps,
                api::teams::create,
                api::teams::delete,
                api::teams::team,
                api::teams::update,
                api::teams::users,
                api::users::me,
                api::users::teams
            ],
        )
        .attach(DbConn::fairing())
}
