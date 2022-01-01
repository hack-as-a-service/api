use bollard::Docker;
use caddy::CaddyClient;
use diesel::{pg::PgConnection as PgConn, prelude::*};
use hyper::{Body, Uri};
use thiserror::Error;
use tokio::sync::broadcast;
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
	#[error("Error while deploying: {0}")]
	DeployError(String),
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type", content = "event")]
pub enum ProvisionerEvent {
	GitClone(String),
	DockerBuild(bollard::models::BuildInfo),
	Deploy(String),
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "deploy")]
pub enum ProvisionerDeployEvent {
	DeployBegin { app_id: i32, image_id: String },
	CreatingNetwork { network_name: String },
	CreatedNetwork { network_id: String },
	UsingExistingNetwork { network_id: String },
	CreatingNewContainer,
	CreatedNewContainer { container_id: String },
	StartingNewContainer,
	StartedNewContainer,
	RetrievingContainerIP,
	RetrievedContainerIP { container_ip: String },
	AddingNewContainerAsUpstream,
	CreatingNewRoute { route_id: String },
	RemovingOldContainerAsUpstream,
	StoppingOldContainer { container_id: String },
	DeletingOldContainer,
	DeployEnd { app_id: i32, app_slug: String },
	Other { log: String },
}

pub use bollard;
pub use caddy;
pub use hyper;

pub type Result<T> = std::result::Result<T, ProvisionerError>;
type DieselResult<T> = std::result::Result<T, diesel::result::Error>;

fn image_id_from_app_id(app_id: i32) -> String {
	format!("haas-apps-{}", app_id)
}

#[async_trait::async_trait]
pub trait DbRunner {
	async fn run<U: Send + 'static>(
		&mut self,
		f: Box<dyn for<'a> FnOnce(&'a mut PgConn) -> DieselResult<U> + Send + 'static>,
	) -> DieselResult<U>;
}

#[async_trait::async_trait]
impl DbRunner for PgConn {
	async fn run<U: Send + 'static>(
		&mut self,
		f: Box<dyn for<'a> FnOnce(&'a mut PgConn) -> DieselResult<U> + Send + 'static>,
	) -> DieselResult<U> {
		f(self)
	}
}

macro_rules! deploy_log {
	($chan:ident, $($args:expr),+) => {
		if let Some(chan) = &$chan {
			chan.send(ProvisionerEvent::Deploy(format!($($args),+))).unwrap();
		}
	}
}

pub struct Provisioner {
	docker: Docker,
	caddy: CaddyClient,
	caddy_name: String,
}

impl Provisioner {
	pub fn connecting_with_local_defaults(
		api_base: caddy::Url,
		caddy_name: String,
	) -> Result<Self> {
		Ok(Self {
			docker: Docker::connect_with_local_defaults()?,
			caddy: CaddyClient::new(api_base)?,
			caddy_name,
		})
	}

	pub async fn tarball_body_for_git_uri(
		uri: &Uri,
		chan: Option<broadcast::Sender<ProvisionerEvent>>,
	) -> Result<Body> {
		use mktemp::Temp;
		use std::process::Stdio;
		use tokio::{fs, process::Command};
		use tokio_stream::StreamExt;
		let clone_dir = Temp::new_path();
		let mut child = Command::new("git")
			.args(&["clone", "--depth=1", &uri.to_string()])
			.arg(clone_dir.as_os_str())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()?;
		let stdout = child.stdout.take().unwrap();
		let stderr = child.stderr.take().unwrap();
		let chan2 = chan.clone();
		tokio::spawn(async move {
			let chan = match chan2 {
				Some(c) => c,
				None => return,
			};
			use tokio::io::{AsyncBufReadExt, BufReader};
			let mut stdout_lines = BufReader::new(stdout).lines();
			let mut stderr_lines = BufReader::new(stderr).lines();
			loop {
				let line = tokio::select! {
					stdout_line = stdout_lines.next_line() => match stdout_line {
						Ok(Some(s)) => s,
						_ => continue,
					},
					stderr_line = stderr_lines.next_line() => match stderr_line {
						Ok(Some(s)) => s,
						_ => continue,
					},
					else => break,
				};
				chan.send(ProvisionerEvent::GitClone(line)).unwrap();
			}
		});
		let status = child.wait().await?;
		if !status.success() {
			return Err(ProvisionerError::GitCloneFailed);
		}
		let archive_path = Temp::new_path();
		let mut child = Command::new("git")
			.args(&["archive", "-o"])
			.arg(archive_path.as_os_str())
			.args(&["HEAD"])
			.current_dir(&clone_dir)
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()?;
		let stdout = child.stdout.take().unwrap();
		let stderr = child.stderr.take().unwrap();
		let chan2 = chan.clone();
		tokio::spawn(async move {
			let chan = match chan2 {
				Some(c) => c,
				None => return,
			};
			use tokio::io::{AsyncBufReadExt, BufReader};
			let mut stdout_lines = BufReader::new(stdout).lines();
			let mut stderr_lines = BufReader::new(stderr).lines();
			loop {
				let line = tokio::select! {
					stdout_line = stdout_lines.next_line() => match stdout_line {
						Ok(Some(s)) => s,
						_ => continue,
					},
					stderr_line = stderr_lines.next_line() => match stderr_line {
						Ok(Some(s)) => s,
						_ => continue,
					},
					else => break,
				};
				chan.send(ProvisionerEvent::GitClone(line)).unwrap();
			}
		});
		let status = child.wait().await?;
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
		app_id: i32,
		app_slug: &str,
		uri: &Uri,
		chan: Option<broadcast::Sender<ProvisionerEvent>>,
	) -> Result<()> {
		use tokio_stream::StreamExt;
		let body = Self::tarball_body_for_git_uri(uri, chan.clone()).await?;
		let mut s = self.docker.build_image(
			bollard::image::BuildImageOptions {
				// FIXME: set limits
				t: image_id_from_app_id(app_id),
				// Deletes intermediate containers created when building,
				// which is what we want
				rm: true,
				forcerm: true,
				labels: [("app.hackclub.app_slug".to_owned(), app_slug.to_owned())].into(),
				..Default::default()
			},
			None,
			Some(body),
		);
		while let Some(ev) = s.next().await {
			let ev = ev?;
			if let Some(chan) = &chan {
				chan.send(ProvisionerEvent::DockerBuild(ev)).unwrap();
			}
		}
		Ok(())
	}

	/// NB: requires that the app's image has been built using [Self#build_image_from_github].
	/// !!! This does not do any privilege checks
	pub async fn deploy_app(
		&self,
		app_id: i32,
		runner: &mut impl DbRunner,
		chan: Option<broadcast::Sender<ProvisionerEvent>>,
	) -> Result<()> {
		use db_models::schema::apps::dsl::{self as apps_dsl, apps, id};
		use db_models::App;
		let image_id = image_id_from_app_id(app_id);
		deploy_log!(
			chan,
			"Deploy begin with app id {}, image id {}",
			app_id,
			image_id
		);
		// 0. Inspect image for exposed port
		let image_metadata = self.docker.inspect_image(&image_id).await?;
		let port = image_metadata
			.config
			.and_then(|c| c.exposed_ports)
			.and_then(|p| p.into_iter().map(|(p, _)| p).find(|p| p.ends_with("tcp")))
			.and_then(|p| p.split('/').next().map(|s| s.to_owned()))
			.and_then(|p| p.parse().ok())
			.unwrap_or(80u16);
		deploy_log!(chan, "Will route traffic to container port {}", port);
		let mut app = runner
			.run(Box::new(move |c| {
				apps.filter(id.eq(app_id)).first::<App>(c)
			}))
			.await?;
		// 1. Get or create the app network
		let network_name = format!("haas_apps_{}", app_id);
		if app.network_id.is_none() {
			deploy_log!(chan, "Creating network with name {}", network_name);
			app.network_id = Some(
				self.docker
					.create_network(bollard::network::CreateNetworkOptions {
						name: network_name.clone(),
						labels: [("app.hackclub.app_slug".to_owned(), app.slug.to_owned())].into(),
						..Default::default()
					})
					.await?
					.id
					.expect("Network create returns id"),
			);
			deploy_log!(
				chan,
				"Created network with id {}",
				app.network_id.as_ref().unwrap()
			);
			deploy_log!(chan, "Adding caddy to the new network...");
			self.docker
				.connect_network(
					app.network_id.as_deref().unwrap(),
					bollard::network::ConnectNetworkOptions::<&str> {
						container: &self.caddy_name,
						..Default::default()
					},
				)
				.await?;
			deploy_log!(chan, "Added caddy to the new network");
		} else {
			deploy_log!(
				chan,
				"Using existing network with id {}",
				app.network_id.as_ref().unwrap()
			);
		}
		// Safe to unwrap: checked None case above
		let network_id = app.network_id.as_deref().unwrap();
		deploy_log!(chan, "Creating new container");
		// 2. Create the new container, attached to the new network
		let new_container = self
			.docker
			.create_container::<&str, _>(
				//Some(bollard::container::CreateContainerOptions {
				//	name: &format!("haas_apps_{}", app_slug),
				//}),
				// FIXME
				None,
				bollard::container::Config {
					image: Some(image_id.as_str()),
					host_config: Some(bollard::service::HostConfig {
						network_mode: Some(network_id.to_owned()),
						..Default::default()
					}),
					labels: Some([("app.hackclub.app_slug", app.slug.as_str())].into()),
					..Default::default()
				},
			)
			.await?
			.id;
		deploy_log!(chan, "Created new container with id {}", new_container);
		deploy_log!(chan, "Starting new container");
		// 2.b. Start the new container
		self.docker
			.start_container::<&str>(&new_container, None)
			.await?;
		deploy_log!(chan, "Started new container");
		deploy_log!(chan, "Retrieving container IP...");
		// 2.c. Get the container's IP
		let new_container_info = self.docker.inspect_container(&new_container, None).await?;
		let new_container_ip = {
			let network_settings =
				new_container_info
					.network_settings
					.as_ref()
					.ok_or_else(|| {
						ProvisionerError::DeployError("Failed to get network settings".to_owned())
					})?;
			let networks = network_settings.networks.as_ref().ok_or_else(|| {
				ProvisionerError::DeployError("Failed to get networks".to_owned())
			})?;
			let network = networks
				.get(&network_name)
				.ok_or_else(|| ProvisionerError::DeployError("Failed to get network".to_owned()))?;
			let ip = network.ip_address.as_ref().ok_or_else(|| {
				ProvisionerError::DeployError("Failed to get IP address".to_owned())
			})?;
			Ok::<_, ProvisionerError>(ip.split('/').next().unwrap().to_string())
		}?;
		deploy_log!(chan, "Retrieved container IP: {}", new_container_ip);
		deploy_log!(chan, "Adding new container as upstream...");
		let upstream = format!("{}:{}", new_container_ip, port);
		//let upstreams_id = format!("haas_apps_{}_upstreams", app_slug);
		let route_id = format!("haas_apps_{}_route", app_id);
		// 3. Update the Caddy upstreams to include the new container upstream
		match {
			let handle = self.caddy.config_by_id(&route_id).appending_path(&[
				"handle",
				"0",
				"upstreams",
				"0",
			]);
			handle
				.put(&caddy::types::Upstream {
					dial: Some(upstream.clone()),
					..Default::default()
				})
				.await
		} {
			// Config updated, good
			Ok(_) => {
				deploy_log!(chan, "Updated upstreams");
			}
			// FIXME: more exact error checking
			// Upstream does not seem to exist... add a new one
			Err(e) => {
				use caddy::types::*;
				deploy_log!(
					chan,
					"Got error {:?}, creating new route with id {}",
					e,
					route_id
				);
				// FIXME: using a static path to routes
				let handle = self
					.caddy
					.config_by_path(&["apps", "http", "servers", "srv0", "routes"]);
				let route = Identified {
					id: Some(route_id.clone()),
					value: Route {
						r#match: Some(vec![HttpMatchersMap {
							// FIXME: domains support
							host: Some(vec![format!("{}.hackclub.app", &app.slug)]),
							..Default::default()
						}
						.into()]),
						handle: Some(vec![HttpHandlers::ReverseProxy(
							reverseproxyHandler {
								upstreams: Some(vec![Upstream {
									dial: Some(upstream.clone()),
									..Default::default()
								}
								.into()]),
								..Default::default()
							}
							.into(),
						)
						.into()]),
						..Default::default()
					},
				};
				//info!("Route: {:?}", route);
				handle.post(&route).await?;
				deploy_log!(chan, "Created new route with upstreams");
			}
		}
		deploy_log!(chan, "FIXME: Waiting 5 seconds for container to be up");
		// 4. Wait 5 seconds or something idk for the new container to be up
		// FIXME
		tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
		// 5. Replace the upstreams list with only the new container ID
		deploy_log!(chan, "Removing old container as upstream...");
		let handle =
			self.caddy
				.config_by_id(&route_id)
				.appending_path(&["handle", "0", "upstreams"]);
		handle
			.patch(&vec![caddy::types::Upstream {
				dial: Some(upstream),
				..Default::default()
			}])
			.await?;
		deploy_log!(chan, "Updated upstreams");
		if let Some(old_container_id) = &app.container_id {
			use bollard::errors::Error as DockerError;
			deploy_log!(chan, "Stopping old container with id {}", old_container_id);
			match self.docker.stop_container(old_container_id, None).await {
				Err(
					DockerError::DockerResponseNotFoundError { .. }
					| DockerError::DockerResponseNotModifiedError { .. },
				) => {
					log::info!("Old container did not exist / already stopped, ignoring");
				}
				e @ Err(_) => return e.map_err(Into::into),
				_ => {}
			}
			deploy_log!(chan, "Deleting old container");
			match self.docker.remove_container(old_container_id, None).await {
				Err(
					DockerError::DockerResponseNotFoundError { .. }
					| DockerError::DockerResponseNotModifiedError { .. },
				) => {
					log::info!("Old container did not exist, ignoring");
				}
				e @ Err(_) => return e.map_err(Into::into),
				_ => {}
			}
		} else {
			deploy_log!(chan, "No old container found to remove");
		}
		// 6. Done! Save the new container id / network id back to the db and return success
		deploy_log!(
			chan,
			"Updating database with new container and network ID..."
		);
		app.container_id = Some(new_container);
		runner
			.run(Box::new({
				let app = app.clone();
				move |c| {
					diesel::update(&app)
						.set((
							apps_dsl::container_id.eq(&app.container_id),
							apps_dsl::network_id.eq(&app.network_id),
						))
						.execute(c)
				}
			}))
			.await?;
		deploy_log!(chan, "Updated database with new container and network ID");
		deploy_log!(
			chan,
			"Successful deploy for app with id {}, slug {}",
			app_id,
			app.slug
		);
		Ok(())
	}
}
