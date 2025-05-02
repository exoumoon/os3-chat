use clap::Parser;
use color_eyre::eyre::Report;
use os3_chat::Settings;
use os3_chat::layers::ErrorLayer;

#[tokio::main]
async fn main() -> Result<(), Report> {
    ErrorLayer.setup()?;

    let settings = Settings::parse();
    os3_chat::run(settings).await?;

    Ok(())
}
