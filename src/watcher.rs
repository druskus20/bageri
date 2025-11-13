use crate::prelude::*;
use color_eyre::eyre::{Context, Result};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

pub fn watch_files(
    watch_patterns: Vec<String>,
    callback: impl Fn() + Send + 'static,
) -> Result<RecommendedWatcher> {
    let (tx, rx) = mpsc::channel();

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            if let Err(e) = tx.send(res) {
                eprintln!("Failed to send watch event: {e}");
            }
        },
        Config::default(),
    )
    .wrap_err("Failed to create file watcher")?;

    // use glob to consolidate watch patterns
    let mut paths_to_watch = Vec::new();
    for pattern in watch_patterns {
        for entry in glob::glob(&pattern).wrap_err("Failed to read glob pattern")? {
            match entry {
                Ok(path) => {
                    if path.is_dir() {
                        paths_to_watch.push(path);
                    } else if let Some(parent) = path.parent() {
                        paths_to_watch.push(parent.to_path_buf());
                    }
                }
                Err(e) => eprintln!("Glob pattern error: {e}"),
            }
        }
    }

    for path in paths_to_watch {
        info!("Watching path: {:?}", path);
        watcher
            .watch(&path, RecursiveMode::Recursive)
            .wrap_err_with(|| format!("Failed to watch path: {:?}", path))?;
    }

    tokio::spawn(async move {
        let mut debounce_timer = None::<tokio::time::Instant>;

        loop {
            match rx.try_recv() {
                Ok(event) => {
                    match event {
                        Ok(_event) => {
                            // Debounce rapid file changes
                            debounce_timer = Some(tokio::time::Instant::now());
                        }
                        Err(e) => {
                            eprintln!("Watch error: {e}");
                        }
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // Check if we should trigger a rebuild
                    if let Some(timer) = debounce_timer {
                        if timer.elapsed() > Duration::from_millis(500) {
                            callback();
                            debounce_timer = None;
                        }
                    }

                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    break;
                }
            }
        }
    });

    Ok(watcher)
}

