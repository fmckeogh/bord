#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    bord::start(bord::Config::new()?).await
}
