#[allow(unused_imports)]
mod prelude {
    pub use crate::log::prelude::*;
    pub use color_eyre::eyre::Context;
    pub use color_eyre::eyre::Result;
}
use prelude::*;

use axum::{Router, routing::get};
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    collections::VecDeque,
    io::{BufRead, BufReader},
    sync::{Arc, Mutex},
    thread,
};
use tower_http::services::ServeDir;

mod cli;
mod config;
mod html;
mod log;
mod watcher;

#[tokio::main]
async fn main() {
    let args = cli::parse_args();
    log::set_max_level(args.log_level());
    log::set_colors_enabled(!args.no_color);
    info!("Hello, world!");

    match args.command {
        cli::Command::Dev(_) => dev().await,
        cli::Command::Build(_) => build().await,
        cli::Command::Clean(_) => clean().await,
    }
    .unwrap();
}

async fn handler() -> &'static str {
    "Hello from Bageri!"
}

async fn dev() -> Result<()> {
    info!("Starting development server...");

    let config = config::Config::load()
        .await
        .wrap_err("Failed to load configuration")?;

    // Run initial build
    build().await?;

    // Start file watcher for src directory
    let _watcher = watcher::watch_files("src", move || {
        info!("Files changed, rebuilding...");
        tokio::spawn(async {
            if let Err(e) = build().await {
                error!("Rebuild failed: {}", e);
            } else {
                info!("Rebuild completed");
            }
        });
    })
    .wrap_err("Failed to start file watcher")?;

    info!("Watching src/ directory for changes");

    let app = Router::new()
        .route(
            "/",
            get(|| async { axum::response::Redirect::permanent("/index.html") }),
        )
        .fallback_service(ServeDir::new(&config.output_dir));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .wrap_err("Failed to bind to address")?;

    info!("Dev server running on http://127.0.0.1:3000");
    info!("Serving files from: {}", config.output_dir);

    axum::serve(listener, app).await.wrap_err("Server error")?;

    Ok(())
}

async fn build() -> Result<()> {
    info!("Building for production...");

    let config = config::Config::load()
        .await
        .wrap_err("Failed to load configuration")?;

    // Create output directory
    tokio::fs::create_dir_all(&config.output_dir)
        .await
        .wrap_err("Failed to create output directory")?;

    // Run pre-build hooks if specified
    if !config.pre_hook.is_empty() {
        info!("Running pre-build hooks...");
        for (i, cmd) in config.pre_hook.iter().enumerate() {
            // Create progress bar for this hook
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::with_template("{spinner:.cyan} [{elapsed_precise}] {msg}")
                    .unwrap_or_else(|_| ProgressStyle::default_spinner()),
            );
            pb.set_message(format!(
                "Running hook {}/{}: {}",
                i + 1,
                config.pre_hook.len(),
                cmd
            ));
            pb.enable_steady_tick(std::time::Duration::from_millis(100));

            // Use spawned process to capture output in real-time
            let mut child = std::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .wrap_err_with(|| format!("Failed to spawn pre-build hook: {}", cmd))?;

            // Read both stdout and stderr in separate threads
            let pb_clone = pb.clone();
            let cmd_name = format!("hook {}/{}", i + 1, config.pre_hook.len());
            let recent_lines = Arc::new(Mutex::new(VecDeque::new()));
            let all_lines = Arc::new(Mutex::new(Vec::new())); // Store all lines for error reporting

            if let Some(stderr) = child.stderr.take() {
                spawn_stderr_reader(
                    stderr,
                    recent_lines.clone(),
                    all_lines.clone(),
                    pb_clone.clone(),
                    cmd_name.clone(),
                );
            }
            if let Some(stdout) = child.stdout.take() {
                spawn_output_reader(
                    stdout,
                    recent_lines.clone(),
                    all_lines.clone(),
                    pb_clone.clone(),
                    cmd_name.clone(),
                );
            }

            let output = child
                .wait_with_output()
                .wrap_err_with(|| format!("Failed to complete pre-build hook: {}", cmd))?;

            if !output.status.success() {
                pb.finish_with_message(format!("Hook {}/{} failed", i + 1, config.pre_hook.len()));

                // Print captured stderr/stdout before exiting
                let lines = all_lines.lock().unwrap();
                if !lines.is_empty() {
                    error!("Hook output:\n{}", lines.join("\n"));
                }

                // Also print the raw output if available
                if !output.stderr.is_empty() {
                    error!("Hook stderr: {}", String::from_utf8_lossy(&output.stderr));
                }
                if !output.stdout.is_empty() {
                    error!("Hook stdout: {}", String::from_utf8_lossy(&output.stdout));
                }

                return Err(color_eyre::eyre::eyre!(
                    "Pre-build hook failed with exit code: {:?}",
                    output.status.code()
                ));
            }

            pb.finish_with_message(format!(
                "Hook {}/{} completed",
                i + 1,
                config.pre_hook.len()
            ));
        }
        info!("All pre-build hooks completed successfully");
    }

    // Generate HTML files for each page
    for (page_name, page) in &config.pages {
        let html_content = html::generate_html(&config, page_name, page);
        let html_filename = if page_name == "index" {
            format!("{}/index.html", config.output_dir)
        } else {
            format!("{}/{}.html", config.output_dir, page_name)
        };

        tokio::fs::write(&html_filename, html_content)
            .await
            .wrap_err_with(|| format!("Failed to write HTML file: {}", html_filename))?;

        info!("Generated HTML file: {}", html_filename);
    }

    info!(
        "Build complete! Static files are in the {} directory.",
        config.output_dir
    );
    Ok(())
}

fn spawn_output_reader<R: std::io::Read + Send + 'static>(
    reader: R,
    recent_lines: Arc<Mutex<VecDeque<String>>>,
    all_lines: Arc<Mutex<Vec<String>>>,
    pb: ProgressBar,
    cmd_name: String,
) {
    thread::spawn(move || {
        let buf_reader = BufReader::new(reader);
        for line in buf_reader.lines().map_while(std::result::Result::ok) {
            if !line.trim().is_empty() {
                // Store in all_lines for complete error reporting
                all_lines.lock().unwrap().push(line.clone());

                let mut lines = recent_lines.lock().unwrap();

                // Keep only the last 5 lines for display
                if lines.len() >= 5 {
                    lines.pop_front();
                }
                lines.push_back(line.clone());

                // Show the last 5 lines (truncated if needed)
                let display_lines: Vec<String> = lines
                    .iter()
                    .map(|line| {
                        if line.chars().count() > 80 {
                            format!(" {}...", line.chars().take(77).collect::<String>())
                        } else {
                            format!(" {}", line)
                        }
                    })
                    .collect();

                let display_text = if display_lines.is_empty() {
                    format!("Running {}...", cmd_name)
                } else {
                    format!("Running {}:\n{}", cmd_name, display_lines.join("\n"))
                };

                pb.set_message(display_text);
            }
        }
    });
}

fn spawn_stderr_reader<R: std::io::Read + Send + 'static>(
    reader: R,
    recent_lines: Arc<Mutex<VecDeque<String>>>,
    all_lines: Arc<Mutex<Vec<String>>>,
    pb: ProgressBar,
    cmd_name: String,
) {
    thread::spawn(move || {
        let buf_reader = BufReader::new(reader);
        for line in buf_reader.lines().map_while(std::result::Result::ok) {
            if !line.trim().is_empty() {
                // Store in all_lines for complete error reporting
                all_lines.lock().unwrap().push(line.clone());

                let mut lines = recent_lines.lock().unwrap();

                // Keep only the last 5 lines for display
                if lines.len() >= 5 {
                    lines.pop_front();
                }
                lines.push_back(line.clone());

                // Show the last 5 lines (truncated if needed)
                let display_lines: Vec<String> = lines
                    .iter()
                    .map(|line| {
                        if line.chars().count() > 80 {
                            format!(" {}...", line.chars().take(77).collect::<String>())
                        } else {
                            format!(" {}", line)
                        }
                    })
                    .collect();

                let display_text = if display_lines.is_empty() {
                    format!("Running {}...", cmd_name)
                } else {
                    format!("Running {}:\n{}", cmd_name, display_lines.join("\n"))
                };

                pb.set_message(display_text);
            }
        }
    });
}

async fn clean() -> Result<()> {
    info!("Cleaning build directories...");

    let config = config::Config::load()
        .await
        .wrap_err("Failed to load configuration")?;

    // Clean bageri output directory
    if tokio::fs::metadata(&config.output_dir).await.is_ok() {
        tokio::fs::remove_dir_all(&config.output_dir)
            .await
            .wrap_err_with(|| format!("Failed to remove directory: {}", config.output_dir))?;
        info!("Cleaned directory: {}", config.output_dir);
    } else {
        info!("Directory {} does not exist, skipping", config.output_dir);
    }

    // Clean .lustre directory (hardcoded since it's used in the hook)
    if tokio::fs::metadata(".lustre").await.is_ok() {
        tokio::fs::remove_dir_all(".lustre")
            .await
            .wrap_err("Failed to remove .lustre directory")?;
        info!("Cleaned directory: .lustre");
    } else {
        info!("Directory .lustre does not exist, skipping");
    }

    info!("Clean complete!");
    Ok(())
}
