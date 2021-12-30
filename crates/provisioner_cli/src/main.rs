use clap::Parser;
use diesel::prelude::*;
use tokio::sync::broadcast;

#[derive(Parser)]
#[clap(version = "0.1")]
struct Opts {
	#[clap(long)]
	id: i32,
	#[clap(subcommand)]
	subcmd: Subcommand,
}

#[derive(Parser)]
enum Subcommand {
	Build {
		#[clap(long)]
		github_uri: String,
		#[clap(long)]
		slug: String,
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
	let provisioner = provisioner::Provisioner::connecting_with_local_defaults(
		caddy_url,
		"caddy-server".to_owned(),
	)?;
	match &opts.subcmd {
		Subcommand::Build {
			github_uri, slug, ..
		} => {
			let (tx, mut rx) = broadcast::channel(10);
			let parsed_uri = github_uri.parse()?;
			let mut build_finish =
				Box::pin(provisioner.build_image_from_github(opts.id, slug, &parsed_uri, Some(tx)));
			loop {
				tokio::select! {
					ev = rx.recv() => {
						let ev = ev.unwrap();
						log::info!("{:?}", ev);
					},
					_ = &mut build_finish => break,
				}
			}
			log::info!("Build done!");
		}
		Subcommand::Deploy { database_url, .. } => {
			let mut conn = diesel::PgConnection::establish(database_url)?;
			let (tx, mut rx) = broadcast::channel(10);
			let mut build_finish = Box::pin(provisioner.deploy_app(opts.id, &mut conn, Some(tx)));
			loop {
				tokio::select! {
					ev = rx.recv() => {
						let ev = ev.unwrap();
						log::info!("{:?}", ev);
					},
					_ = &mut build_finish => break,
				}
			}
			log::info!("Deploy done!");
		}
	}
	Ok(())
}
