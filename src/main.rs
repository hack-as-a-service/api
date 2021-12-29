#[macro_use]
extern crate rocket;

#[macro_use]
extern crate lazy_static;

use diesel::prelude::*;
use dotenv::dotenv;
use rocket::tokio::{net::UdpSocket, sync::RwLock};
use rocket_sync_db_pools::database;
use trust_dns_client::{client::AsyncClient, udp::UdpClientStream};

mod api;
mod auth;
mod slack;
mod utils;

#[database("db")]
pub struct DbConn(PgConnection);

#[get("/openapi.yaml")]
async fn openapi() -> &'static str {
	include_str!("../openapi/openapi.yaml")
}

#[launch]
async fn rocket() -> _ {
	dotenv().ok();

	// Instantiate a DNS client for domain verification
	let stream = UdpClientStream::<UdpSocket>::new(([1, 1, 1, 1], 53).into());
	let (dns_client, bg) = AsyncClient::connect(stream)
		.await
		.expect("Error instantiating DNS client");

	rocket::tokio::spawn(bg);

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
				api::domains::verify,
				api::oauth::create_device_authorization,
				api::oauth::device_authorization,
				api::oauth::device_approve,
				api::oauth::device_reject,
				api::oauth::token,
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
		.manage(RwLock::new(dns_client))
}
