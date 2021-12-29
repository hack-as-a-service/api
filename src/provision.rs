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
	#[serde(with = "url_serializer")]
	caddy_api_base: provisioner::caddy::Url,
	caddy_container_name: String,
}

mod url_serializer {
	use provisioner::caddy::Url;
	use serde::de::{
		Deserializer, Error as DeError, Unexpected as DeUnexpected, Visitor as DeVisitor,
	};
	use serde::ser::Serializer;

	pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Url, D::Error> {
		struct UrlVisitor;

		impl<'de> DeVisitor<'de> for UrlVisitor {
			type Value = Url;

			fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				v.parse()
					.map_err(|_| DeError::invalid_value(DeUnexpected::Str(v), &self))
			}

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(formatter, "a URI")
			}
		}

		de.deserialize_str(UrlVisitor)
	}

	pub fn serialize<S: Serializer>(url: &Url, ser: S) -> Result<S::Ok, S::Error> {
		ser.serialize_str(&url.to_string())
	}
}

pub struct ProvisionerManager {
	provisioner: Provisioner,
	event_channels: HashMap<i32, Sender<ProvisionerEvent2>>,
}

impl ProvisionerManager {
	pub fn from_figment(f: &rocket::figment::Figment) -> provisioner::Result<Self> {
		let c = f
			.extract_inner::<ProvisionerConfig>("provisioner")
			.expect("Failed to extract config from figment");
		Ok(Self {
			provisioner: Provisioner::connecting_with_local_defaults(
				c.caddy_api_base,
				c.caddy_container_name,
			)?,
			event_channels: Default::default(),
		})
	}

	pub async fn create_build(
		self: Arc<Self>,
		conn: Arc<DbConn>,
		git_uri: Uri,
		app_id: i32,
		app_slug: &str,
	) -> diesel::QueryResult<i32> {
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
						conn2
							.run(move |c| {
								// Diesel doesn't support array_append
								diesel::sql_query(
								"UPDATE builds SET events = array_append(events, ?) WHERE id = ?",
							)
							.bind::<diesel::sql_types::Text, _>(serde_json::to_string(&ev).unwrap())
							.bind::<diesel::sql_types::Integer, _>(build_id)
							.execute(c)
							.unwrap();
							})
							.await;
					}
					Err(broadcast::error::RecvError::Closed) => break,
					_ => {}
				}
			}
		});
		// Start the build / deploy
		tokio::spawn(async move {
			let (tx2, mut rx2) = broadcast::channel(10);
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
			let br = self
				.provisioner
				.build_image_from_github(app_id, &app_slug, &git_uri, Some(tx2.clone()))
				.await;
			if let Err(e) = br {
				tx.send(ProvisionerEvent2::make(Err(e.to_string())))
					.unwrap();
			}
			let dr = self
				.provisioner
				.deploy_app(app_id, &mut &runner, Some(tx2.clone()))
				.await;
			if let Err(e) = dr {
				tx.send(ProvisionerEvent2::make(Err(e.to_string())))
					.unwrap();
			}
		});
		Ok(5)
	}
}
