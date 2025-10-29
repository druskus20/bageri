mod prelude {
    pub use crate::log::prelude::*;
    pub use color_eyre::eyre::Context;
    pub use color_eyre::{Report, eyre::Result};
}
use prelude::*;

mod cli;
mod log;

#[tokio::main]
async fn main() {
    let args = cli::parse_args();
    log::set_max_level(args.log_level());
    log::set_colors_enabled(!args.no_color);
    info!("Hello, world!");

    match args.command {
        cli::Command::Dev(_) => dev().await,
        cli::Command::Build(_) => build().await,
    }
    .unwrap();
}

async fn handler() -> &'static str {
    "Hello from Bagery!"
}

async fn dev() -> Result<()> {
    use axum::{Router, response::Html, routing::get};

    info!("Starting development server...");

    let app = Router::new().route("/", get(handler));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .wrap_err("Failed to bind to address")?;

    info!("Dev server running on http://127.0.0.1:3000");

    axum::serve(listener, app).await.wrap_err("Server error")?;

    Ok(())
}

async fn build() -> Result<()> {
    info!("Building for production...");
    // Build logic here
    Ok(())
}
