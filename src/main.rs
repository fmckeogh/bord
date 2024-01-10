use {
    clap::Parser,
    color_eyre::eyre::Context,
    std::{fs::File, path::PathBuf},
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to JSON config file
    #[arg(long)]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    let file = File::open(&args.config).wrap_err(format!(
        "Failed to open configuration file @ {:?}",
        &args.config
    ))?;

    let config = serde_json::from_reader(file).wrap_err("Failed to parse JSON in config file")?;

    bord::start(config).await
}
