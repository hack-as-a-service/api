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
	Deploy(ProvisionerDeployEvent),
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

impl From<ProvisionerDeployEvent> for ProvisionerEvent {
	fn from(ev: ProvisionerDeployEvent) -> Self {
		Self::Deploy(ev)
	}
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
		if let Some(chan) = &chan {
			chan.send(
				ProvisionerDeployEvent::DeployBegin {
					app_id,
					image_id: image_id.clone(),
				}
				.into(),
			)
			.unwrap();
		}
		let mut app = runner
			.run(Box::new(move |c| {
				apps.filter(id.eq(app_id)).first::<App>(c)
			}))
			.await?;
		// 1. Get or create the app network
		let network_name = format!("haas_apps_{}", app_id);
		if app.network_id.is_none() {
			if let Some(chan) = &chan {
				chan.send(
					ProvisionerDeployEvent::CreatingNetwork {
						network_name: network_name.clone(),
					}
					.into(),
				)
				.unwrap();
			}
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
			if let Some(chan) = &chan {
				chan.send(
					ProvisionerDeployEvent::CreatedNetwork {
						network_id: app.network_id.as_ref().unwrap().clone(),
					}
					.into(),
				)
				.unwrap();
				chan.send(
					ProvisionerDeployEvent::Other {
						log: "Adding caddy to the new network...".to_owned(),
					}
					.into(),
				)
				.unwrap();
			}
			self.docker
				.connect_network(
					app.network_id.as_deref().unwrap(),
					bollard::network::ConnectNetworkOptions::<&str> {
						container: &self.caddy_name,
						..Default::default()
					},
				)
				.await?;
			if let Some(chan) = &chan {
				chan.send(
					ProvisionerDeployEvent::Other {
						log: "Added caddy to the new network".to_owned(),
					}
					.into(),
				)
				.unwrap();
			}
		} else if let Some(chan) = &chan {
			chan.send(
				ProvisionerDeployEvent::UsingExistingNetwork {
					network_id: app.network_id.as_ref().unwrap().clone(),
				}
				.into(),
			)
			.unwrap();
		}
		// Safe to unwrap: checked None case above
		let network_id = app.network_id.as_deref().unwrap();
		if let Some(chan) = &chan {
			chan.send(ProvisionerDeployEvent::CreatingNewContainer.into())
				.unwrap();
		}
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
		if let Some(chan) = &chan {
			chan.send(
				ProvisionerDeployEvent::CreatedNewContainer {
					container_id: new_container.clone(),
				}
				.into(),
			)
			.unwrap();
			chan.send(ProvisionerDeployEvent::StartingNewContainer.into())
				.unwrap();
		}
		// 2.b. Start the new container
		self.docker
			.start_container::<&str>(&new_container, None)
			.await?;
		if let Some(chan) = &chan {
			chan.send(ProvisionerDeployEvent::StartedNewContainer.into())
				.unwrap();
			chan.send(ProvisionerDeployEvent::RetrievingContainerIP.into())
				.unwrap();
		}
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
		if let Some(chan) = &chan {
			chan.send(
				ProvisionerDeployEvent::RetrievedContainerIP {
					container_ip: new_container_ip.clone(),
				}
				.into(),
			)
			.unwrap();
			chan.send(ProvisionerDeployEvent::AddingNewContainerAsUpstream.into())
				.unwrap();
		}
		// FIXME: currently we assume the port is 80
		let upstream = format!("{}:80", new_container_ip);
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
				//info!("Updated upstreams");
			}
			// FIXME: more exact error checking
			// Upstream does not seem to exist... add a new one
			Err(_) => {
				use caddy::types::*;
				if let Some(chan) = &chan {
					chan.send(
						ProvisionerDeployEvent::CreatingNewRoute {
							route_id: route_id.clone(),
						}
						.into(),
					)
					.unwrap();
				}
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
				//info!("Created route with upstreams");
			}
		}
		//info!("Waiting 5 seconds for container to be up (FIXME)");
		// 4. Wait 5 seconds or something idk for the new container to be up
		// FIXME
		tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
		// 5. Replace the upstreams list with only the new container ID
		if let Some(chan) = &chan {
			chan.send(ProvisionerDeployEvent::RemovingOldContainerAsUpstream.into())
				.unwrap();
		}
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
		//info!("Updated upstreams");
		if let Some(old_container_id) = &app.container_id {
			if let Some(chan) = &chan {
				chan.send(
					ProvisionerDeployEvent::StoppingOldContainer {
						container_id: old_container_id.clone(),
					}
					.into(),
				)
				.unwrap();
			}
			self.docker.stop_container(old_container_id, None).await?;
			if let Some(chan) = &chan {
				chan.send(ProvisionerDeployEvent::DeletingOldContainer.into())
					.unwrap();
			}
			self.docker.remove_container(old_container_id, None).await?;
		} else {
			//info!("None found");
		}
		// 6. Done! Save the new container id / network id back to the db and return success
		//info!("Updating database with new container and network ID");
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
		//info!("Updated database with new container and network ID");
		if let Some(chan) = &chan {
			chan.send(
				ProvisionerDeployEvent::DeployEnd {
					app_id,
					app_slug: app.slug.clone(),
				}
				.into(),
			)
			.unwrap();
		}
		Ok(())
	}
}
