#[macro_use]
extern crate diesel;

#[macro_use]
extern crate rocket;

use diesel::prelude::*;
use dotenv::dotenv;
use rocket_sync_db_pools::database;

mod api;
mod auth;
mod models;
mod schema;
mod slack;

#[database("db")]
pub struct DbConn(PgConnection);

#[launch]
fn rocket() -> _ {
    dotenv().ok();

    rocket::build()
        .mount(
            "/api",
            routes![
                api::auth::login,
                api::auth::logout,
                api::auth::code,
                api::users::me
            ],
        )
        .attach(DbConn::fairing())
}
