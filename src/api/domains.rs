use diesel::{
	prelude::*,
	result::{
		DatabaseErrorKind::UniqueViolation,
		Error::{DatabaseError, NotFound},
	},
};
use rocket::{http::Status, serde::json::Json, tokio::sync::RwLock, State};

use db_models::{App, Domain, NewDomain, Team, TeamUser};
use trust_dns_client::client::AsyncClient;

use crate::{
	auth::AuthUser,
	utils::domain::{validate_domain, verify_domain},
	DbConn,
};

#[post("/apps/<app_slug>/domains", data = "<domain>")]
pub async fn create(
	app_slug: String,
	domain: Json<NewDomain>,
	user: AuthUser,
	conn: DbConn,
) -> Result<Json<Domain>, Status> {
	if !validate_domain(&domain.domain) {
		return Err(Status::UnprocessableEntity);
	}

	conn.run(move |c| {
		use db_models::schema::apps::dsl::{apps, slug};
		use db_models::schema::domains::dsl::domains;
		use db_models::schema::team_users::dsl::{team_users, user_id};
		use db_models::schema::teams::dsl::teams;

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

#[post("/domains/<id>/verify")]
pub async fn verify(
	id: i32,
	user: AuthUser,
	dns: &State<RwLock<AsyncClient>>,
	conn: DbConn,
) -> Result<Json<Domain>, Status> {
	// Fetch the domain
	let mut domain = conn
		.run(move |c| {
			use db_models::schema::apps::dsl::apps;
			use db_models::schema::domains::dsl::{domains, id as domain_id};
			use db_models::schema::team_users::dsl::{team_users, user_id};
			use db_models::schema::teams::dsl::teams;

			let domain = domains
				.inner_join(apps.inner_join(teams.inner_join(team_users)))
				.filter(domain_id.eq(id).and(user_id.eq(user.id)))
				.first::<(Domain, (App, (Team, TeamUser)))>(c)
				.map_err(|e| {
					if e == NotFound {
						Status::NotFound
					} else {
						Status::InternalServerError
					}
				})?;

			Ok(domain.0)
		})
		.await?;

	// Check its DNS config
	let is_verified = verify_domain(&mut *dns.write().await, &domain.domain)
		.await
		.map_err(|_| Status::InternalServerError)?;

	// Update accordingly
	domain = conn
		.run(move |c| {
			use db_models::schema::domains::dsl::{domains, id as domain_id, verified};

			diesel::update(domains.filter(domain_id.eq(domain.id)))
				.set(verified.eq(is_verified))
				.get_result::<Domain>(c)
				.map_err(|e| {
					if let DatabaseError(UniqueViolation, _) = e {
						Status::Conflict
					} else {
						Status::InternalServerError
					}
				})
		})
		.await?;

	Ok(Json(domain))
}
