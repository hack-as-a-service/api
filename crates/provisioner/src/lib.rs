use bollard::Docker;
use hyper::{Body, Uri};
use thiserror::Error;
use tokio_stream::Stream;

#[derive(Error, Debug)]
pub enum ProvisionerError {
	#[error("Docker error: {0}")]
	Docker(#[from] bollard::errors::Error),
	#[error("Hyper error: {0}")]
	Hyper(#[from] hyper::Error),
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
}

impl Provisioner {
	pub fn connect_with_local_defaults() -> Result<Self> {
		Ok(Self {
			docker: Docker::connect_with_local_defaults()?,
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
}
