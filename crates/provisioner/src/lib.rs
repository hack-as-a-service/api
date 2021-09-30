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

pub struct Provisioner {
    docker: Docker,
}

impl Provisioner {
    pub fn connect_with_local_defaults() -> Result<Self> {
        Ok(Self {
            docker: Docker::connect_with_local_defaults()?,
        })
    }

    // pub async fn tarball_body_for_github_url(uri: &Uri, rev: Option<&str>) -> Result<Body> {
    //     if uri.authority().map(|x| x.host()) != Some("github.com") {
    //         return Err(ProvisionerError::InvalidGitHubURI);
    //     }
    //     let (owner, repo) = {
    //         let mut s = uri.path().split("/").skip(1);
    //         let a = s.next().ok_or(ProvisionerError::InvalidGitHubURI)?;
    //         let b = s.next().ok_or(ProvisionerError::InvalidGitHubURI)?;
    //         (a, b)
    //     };
    //     let new_uri = Uri::builder()
    //         .scheme("https")
    //         .authority("api.github.com")
    //         .path_and_query(format!("/repos/{}/{}/tarball/{}", owner, repo, rev.unwrap_or("")))
    //         .build()?;
    //     //let new_uri = format!("https://api.github.com/repos/{}/{}/tarball/{}", owner, repo, rev.unwrap_or("")).parse().unwrap();
    //     println!("url is {}", new_uri);
    //     let https = hyper_tls::HttpsConnector::new();
    //     let client = hyper::Client::builder().build::<_, Body>(https);
    //     let req = hyper::Request::get(&new_uri)
    //         .header("Accept", "*/*")
    //         .header("User-Agent", "curl 7")
    //         .body(Body::empty())
    //         .unwrap();
    //     let response = client.request(req).await?;
    //     if !response.status().is_redirection() {
    //         println!("response is not redirect, got {}", response.status());
    //         return Err(ProvisionerError::RequestFailed);
    //     }
    //     // Follow `Location` header again...
    //     let response2 = client.get(response.headers().get(hyper::http::header::LOCATION).unwrap().to_str().unwrap().parse().unwrap()).await?;
    //     if response2.status().is_success() {
    //         Ok(response2.into_body())
    //     } else {
    //         Err(ProvisionerError::RequestFailed)
    //     }
    // }

    pub async fn tarball_body_for_git_uri(uri: &Uri) -> Result<Body> {
        use mktemp::Temp;
        use tokio::{fs, process::Command};
        use tokio_stream::StreamExt;
        use tokio_util::codec::Decoder;
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
        //fs::remove_dir_all(&clone_dir).await?;
        let f = fs::File::open(archive_path).await?;
        let codec = tokio_util::codec::BytesCodec::new();
        let mapped_stream = codec.framed(f).map(|i| {
            i.map(|b| b.freeze())
                // Has to be coerced for Into<Body>
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync + 'static>)
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
        &mut self,
        image_id: &str,
        uri: &Uri,
    ) -> Result<
        impl Stream<Item = std::result::Result<bollard::models::BuildInfo, bollard::errors::Error>>,
    > {
        let body = Self::tarball_body_for_git_uri(uri).await?;
        Ok(self.docker.build_image(
            bollard::image::BuildImageOptions {
                // FIXME: set limits
                t: format!("haas-apps/{}", image_id),
                // Deletes intermediate containers created when building,
                // which is what we want
                rm: true,
                forcerm: true,
                ..Default::default()
            },
            None,
            Some(body),
        ))
        //while let Some(s) = stream.try_next().await? {
        //    // empty
        //}
        //Ok(())
    }
}
