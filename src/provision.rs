#![allow(dead_code)] // Remove once we have the API routes for this

use std::collections::HashMap;
use std::sync::Arc;

use crate::DbConn;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use provisioner::{Provisioner, ProvisionerEvent};
use tokio::sync::broadcast::{self, Sender};

pub use provisioner::hyper::Uri;

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ProvisionerEvent2 {
	ts: NaiveDateTime,
	#[serde(flatten)]
	event: Result<ProvisionerEvent, String>,
}

impl ProvisionerEvent2 {
	pub fn make(event: Result<ProvisionerEvent, String>) -> Self {
		let ts = chrono::Utc::now().naive_utc();
		Self { ts, event }
	}
}

struct PooledDbRunner {
	c: Arc<DbConn>,
}

#[rocket::async_trait]
impl provisioner::DbRunner for &PooledDbRunner {
	async fn run<U: Send + 'static>(
		&mut self,
		f: Box<
			dyn for<'a> FnOnce(&'a mut PgConnection) -> Result<U, diesel::result::Error>
				+ Send
				+ 'static,
		>,
	) -> Result<U, diesel::result::Error> {
		self.c.run(f).await
	}
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ProvisionerConfig {
	#[serde(with = "crate::utils::url_serializer")]
	caddy_api_base: provisioner::caddy::Url,
	caddy_container_name: String,
}

pub struct ProvisionerManager {
	provisioner: Arc<Provisioner>,
	event_channels: HashMap<i32, Sender<ProvisionerEvent2>>,
}

impl ProvisionerManager {
	pub fn from_figment(f: &rocket::figment::Figment) -> provisioner::Result<Self> {
		let c = f
			.extract_inner::<ProvisionerConfig>("provisioner")
			.expect("Failed to extract config from figment");
		Ok(Self {
			provisioner: Arc::new(Provisioner::connecting_with_local_defaults(
				c.caddy_api_base,
				c.caddy_container_name,
			)?),
			event_channels: Default::default(),
		})
	}

	pub async fn create_build(
		&mut self,
		conn: Arc<DbConn>,
		git_uri: Uri,
		app_id: i32,
		app_slug: &str,
	) -> diesel::QueryResult<db_models::Build> {
		use db_models::schema::builds::dsl::builds;
		use db_models::{Build, NewBuild};
		let app_slug = app_slug.to_owned();
		let build = conn
			.run(move |c| {
				diesel::insert_into(builds)
					.values(NewBuild { app_id })
					.get_result::<Build>(c)
			})
			.await?;
		let build_id = build.id;
		let (tx, mut rx) = broadcast::channel(10);
		let conn2 = Arc::clone(&conn);
		// Receive build events and append them to the db
		tokio::spawn(async move {
			loop {
				match rx.recv().await {
					Ok(ev) => {
						// please don't add overhead please don't add
						// overhead
						if let Ok(debug_ev) = serde_json::to_string(&ev) {
							println!("debug: build {} event: {}", build_id, debug_ev);
						}
						conn2
							.run(move |c| {
								use db_models::schema::builds::dsl::{events, id};
								use diesel::{dsl::sql, sql_types::Text};

								let q = diesel::update(builds).filter(id.eq(build_id)).set(
									// diesel doesn't have a
									// query builder for
									// array_append
									events.eq(sql("array_append(events, ")
										.bind::<Text, _>(serde_json::to_string(&ev).unwrap())
										.sql(")")),
								);
								#[cfg(debug_assertions)]
								println!(
									"query = {}",
									diesel::debug_query::<diesel::pg::Pg, _>(&q)
								);
								q.execute(c).unwrap();
							})
							.await;
					}
					Err(broadcast::error::RecvError::Closed) => break,
					_ => {}
				}
			}
		});
		let (tx2, mut rx2) = broadcast::channel(10);
		// Start the build / deploy
		let provisioner = Arc::clone(&self.provisioner);
		tokio::spawn({
			let tx = tx.clone();
			async move {
				let tx_clone = tx.clone();
				tokio::spawn(async move {
					loop {
						match rx2.recv().await {
							Ok(ev) => {
								tx_clone.send(ProvisionerEvent2::make(Ok(ev))).unwrap();
							}
							Err(broadcast::error::RecvError::Closed) => break,
							_ => {}
						}
					}
				});
				let runner = PooledDbRunner { c: conn.clone() };
				let br = provisioner
					.build_image_from_github(app_id, &app_slug, &git_uri, Some(tx2.clone()))
					.await;
				if let Err(e) = br {
					tx.send(ProvisionerEvent2::make(Err(e.to_string())))
						.unwrap();
				} else {
					let dr = provisioner
						.deploy_app(app_id, &mut &runner, Some(tx2.clone()))
						.await;
					if let Err(e) = dr {
						tx.send(ProvisionerEvent2::make(Err(e.to_string())))
							.unwrap();
					}
				}
				conn.run(move |c| {
					use db_models::schema::builds::dsl::{ended_at, id};

					diesel::update(builds)
						.filter(id.eq(build_id))
						.set(ended_at.eq(chrono::Utc::now().naive_utc()))
						.execute(c)
						.unwrap();
				})
				.await;
			}
		});
		self.event_channels.insert(build.id, tx);
		Ok(build)
	}

	pub fn receiver_for_build(&self, id: i32) -> Option<broadcast::Receiver<ProvisionerEvent2>> {
		self.event_channels
			.get(&id)
			.map(broadcast::Sender::subscribe)
	}
}
