#[macro_use]
extern crate diesel;

#[macro_use]
extern crate rocket;

use diesel::prelude::*;
use dotenv::dotenv;
use rocket::http::CookieJar;
use rocket_sync_db_pools::database;

mod api;
mod models;
mod schema;
mod slack;

use api::*;

#[database("db")]
pub struct DbConn(PgConnection);

#[get("/")]
pub fn index(cookies: &CookieJar<'_>) -> Option<String> {
    cookies
        .get("haas_token")
        .map(|crumb| format!("{}", crumb.value()))
}

#[launch]
fn rocket() -> _ {
    dotenv().ok();

    rocket::build()
        .mount("/", routes![index])
        .mount("/api", routes![auth::login, auth::logout, auth::code])
        .attach(DbConn::fairing())
}
