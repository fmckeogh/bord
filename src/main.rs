#[tokio::main(flavor = "multi_thread")]
async fn main() -> color_eyre::Result<()> {
    // setup colorful backtraces
    color_eyre::install()?;
    // retrieve config from environment, start bord with said config
    bord::start(bord::Config::from_env()?).await
}
