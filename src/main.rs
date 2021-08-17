#[macro_use]
extern crate diesel;

#[macro_use]
extern crate rocket;

use diesel::prelude::*;
use dotenv::dotenv;
use rocket_sync_db_pools::database;

mod api;
mod models;
mod schema;
mod slack;

use api::*;

#[database("db")]
struct DbConn(PgConnection);

#[launch]
fn rocket() -> _ {
    dotenv().ok();

    rocket::build()
        .mount("/api", routes![auth::login, auth::code])
        .attach(DbConn::fairing())
}
