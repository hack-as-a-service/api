use futures_util::TryStreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let image_id = args.next().expect("first arg is image id");
    let url = args.next().expect("second arg is github uri");
    let mut p = provisioner::Provisioner::connect_with_local_defaults()?;
    let mut s = p.build_image_from_github(&image_id, &url.parse()?).await?;
    while let Some(s2) = s.try_next().await? {
        println!("{:?}", s2);
    }
    println!("Build done!");
    Ok(())
}
