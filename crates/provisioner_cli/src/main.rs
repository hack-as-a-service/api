use clap::Parser;
use diesel::prelude::*;
use futures_util::TryStreamExt;

#[derive(Parser)]
#[clap(version = "0.1")]
struct Opts {
	#[clap(long)]
	slug: String,
	#[clap(subcommand)]
	subcmd: Subcommand,
}

#[derive(Parser)]
enum Subcommand {
	Build {
		#[clap(long)]
		github_uri: String,
	},
	Deploy {
		#[clap(long)]
		database_url: String,
	},
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	pretty_env_logger::init();
	let opts = Opts::parse();
	let caddy_url = provisioner::caddy::Url::parse("http://localhost:2019/")?;
	let mut provisioner = provisioner::Provisioner::connecting_with_local_defaults(
		caddy_url,
		"caddy-server".to_owned(),
	)?;
	match &opts.subcmd {
		Subcommand::Build { github_uri, .. } => {
			let mut s = provisioner
				.build_image_from_github(&opts.slug, &github_uri.parse()?)
				.await?;
			while let Some(s2) = s.try_next().await? {
				log::info!("{:?}", s2);
			}
			log::info!("Build done!");
		}
		Subcommand::Deploy { database_url, .. } => {
			let conn = diesel::PgConnection::establish(database_url)?;
			provisioner.deploy_app(&opts.slug, &conn).await?;
			log::info!("Deploy done!");
		}
	}
	Ok(())
}
