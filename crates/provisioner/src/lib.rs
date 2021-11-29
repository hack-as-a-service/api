use bollard::Docker;
use caddy::CaddyClient;
use diesel::{pg::PgConnection as PgConn, prelude::*};
use hyper::{Body, Uri};
use log::info;
use thiserror::Error;
use tokio_stream::Stream;

#[derive(Error, Debug)]
pub enum ProvisionerError {
	#[error("Docker error: {0}")]
	Docker(#[from] bollard::errors::Error),
	#[error("Hyper error: {0}")]
	Hyper(#[from] hyper::Error),
	#[error("Diesel error: {0}")]
	Diesel(#[from] diesel::result::Error),
	#[error("Caddy error: {0}")]
	Caddy(#[from] caddy::CaddyError),
	#[error("http error: {0}")]
	Http(#[from] hyper::http::Error),
	#[error("IO error: {0}")]
	IO(#[from] std::io::Error),
	#[error("Git clone failed")]
	GitCloneFailed,
}

pub use bollard;
pub use hyper;

type Result<T> = std::result::Result<T, ProvisionerError>;

fn image_id_from_slug(slug: &str) -> String {
	format!("haas-apps/{}", slug)
}

pub struct Provisioner {
	docker: Docker,
	caddy: CaddyClient,
}

impl Provisioner {
	pub fn connect_with_local_defaults_and_api_base(api_base: caddy::Url) -> Result<Self> {
		Ok(Self {
			docker: Docker::connect_with_local_defaults()?,
			caddy: CaddyClient::new(api_base)?,
		})
	}

	pub async fn tarball_body_for_git_uri(uri: &Uri) -> Result<Body> {
		use mktemp::Temp;
		use tokio::{fs, process::Command};
		use tokio_stream::StreamExt;
		let clone_dir = Temp::new_path();
		let status = Command::new("git")
			.args(&["clone", "--depth=1", &uri.to_string()])
			.arg(clone_dir.as_os_str())
			.status()
			.await?;
		if !status.success() {
			return Err(ProvisionerError::GitCloneFailed);
		}
		let archive_path = Temp::new_path();
		let status = Command::new("git")
			.args(&["archive", "-o"])
			.arg(archive_path.as_os_str())
			.args(&["HEAD"])
			.current_dir(&clone_dir)
			.status()
			.await?;
		if !status.success() {
			return Err(ProvisionerError::GitCloneFailed);
		}
		let f = fs::File::open(archive_path).await?;
		let stream = tokio_util::io::ReaderStream::new(f);
		let mapped_stream = stream.map(|i| {
			// Has to be coerced for Into<Body>
			i.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync + 'static>)
		});
		// Type that Into<Body> expects
		let s2: Box<
			dyn Stream<
					Item = std::result::Result<
						hyper::body::Bytes,
						Box<dyn std::error::Error + Send + Sync + 'static>,
					>,
				> + Send
				+ 'static,
		> = Box::new(mapped_stream);
		Ok(s2.into())
	}

	pub async fn build_image_from_github(
		&self,
		app_slug: &str,
		uri: &Uri,
	) -> Result<
		impl Stream<Item = std::result::Result<bollard::models::BuildInfo, bollard::errors::Error>>,
	> {
		let body = Self::tarball_body_for_git_uri(uri).await?;
		Ok(self.docker.build_image(
			bollard::image::BuildImageOptions {
				// FIXME: set limits
				t: image_id_from_slug(app_slug),
				// Deletes intermediate containers created when building,
				// which is what we want
				rm: true,
				forcerm: true,
				..Default::default()
			},
			None,
			Some(body),
		))
	}

	/// NB: requires that the app's image has been built using [Self#build_image_from_github].
	/// !!! This does not do any privilege checks
	pub async fn deploy_app(&self, app_slug: &str, c: &PgConn) -> Result<()> {
		use db_models::schema::apps::dsl::{self as apps_dsl, apps, slug};
		use db_models::App;
		let image_id = image_id_from_slug(app_slug);
		info!("Deploying app by slug {} (image {})", app_slug, image_id);
		let mut app = apps.filter(slug.eq(app_slug)).first::<App>(c)?;
		// 1. Get or create the app network
		if app.network_id.is_none() {
			let network_name = format!("haas_apps_{}", app_slug);
			info!("Creating new network {}", network_name);
			app.network_id = Some(
				self.docker
					.create_network(bollard::network::CreateNetworkOptions {
						name: network_name,
						..Default::default()
					})
					.await?
					.id
					.expect("Network create returns id"),
			);
			info!("Created network, id = {}", app.network_id);
		} else {
			info!(
				"Using existing network id {}",
				app.network_id.as_deref().unwrap()
			);
		}
		// Safe to unwrap: checked None case above
		let network_id = app.network_id.as_deref().unwrap();
		info!("Creating new container");
		// 2. Create the new container, attached to the new network
		let new_container = self
			.docker
			.create_container::<&str, _>(
				bollard::container::CreateContainerOptions {
					name: Some(&format!("haas_apps_{}", app_slug)),
				},
				bollard::container::Config {
					image: Some(image_id.as_str()),
					host_config: Some(bollard::service::HostConfig {
						network_mode: Some(network_id.to_owned()),
						..Default::default()
					}),
					..Default::default()
				},
			)
			.await?
			.id;
		info!("Created new container, id = {}, starting", new_container);
		// 2.b. Start the new container
		self.docker.start_container(&new_container, None).await?;
		info!("Started container");
		// FIXME: currently we assume the port is 80
		let upstream = format!("{}:80", new_container);
		let upstreams_id = format!("haas_apps_{}_upstreams", app_slug);
		// 3. Update the Caddy upstreams to include the new container upstream
		info!("Updating upstreams (1)");
		match {
			let handle = self.caddy.config_by_id(&upstreams_id);
			handle.post(&upstream.as_str()).await
		} {
			// Config updated, good
			Ok(_) => {
				info!("Updated upstreams");
			}
			// FIXME: more exact error checking
			// Upstream does not seem to exist... add a new one
			Err(_) => {
				info!("Upstreams not found, creating route");
				use caddy::types::*;
				// FIXME: using a static path to routes
				let handle = self
					.caddy
					.config_by_path(&["apps", "http", "servers", "srv0", "routes"]);
				let route = Identifiable::Identified {
					id: format!("haas_apps_{}_route", app_slug),
					value: Route {
						r#match: Some(
							vec![HttpMatchersMap {
								// FIXME: domains support
								host: Some(
									vec![format!("{}.hackclub.app", app_slug).into()].into(),
								),
								..Default::default()
							}
							.into()]
							.into(),
						),
						handle: Some(
							vec![HttpHandlers::ReverseProxy(
								reverseproxyHandler {
									upstreams: Some(Identifiable::Identified {
										id: upstreams_id.clone(),
										value: vec![Upstream {
											dial: Some(upstream.clone().into()),
											..Default::default()
										}
										.into()],
									}),
									..Default::default()
								}
								.into(),
							)
							.into()]
							.into(),
						),
						..Default::default()
					},
				};
				handle.post(&route).await?;
				info!("Created route with upstreams");
			}
		}
		info!("Waiting 5 seconds for container to be up (FIXME)");
		// 4. Wait 5 seconds or something idk for the new container to be up
		// FIXME
		tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
		// 5. Replace the upstreams list with only the new container ID
		info!("Updating upstreams (2)");
		let handle = self.caddy.config_by_id(&upstreams_id);
		handle
			.patch(&vec![caddy::types::Upstream {
				dial: Some(upstream.into()),
				..Default::default()
			}])
			.await?;
		info!("Updated upstreams");
		// 6. Done! Save the new container id / network id back to the db and return success
		info!("Updating database with new container and network ID");
		diesel::update(&app)
			.set((
				apps_dsl::container_id.eq(&app.container_id),
				apps_dsl::network_id.eq(&app.network_id),
			))
			.execute(c)?;
		info!("Updated database with new container and network ID");
		info!("Successfully deployed app with slug {}", app_slug);
		Ok(())
	}
}
