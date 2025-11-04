use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;
use color_eyre::eyre::{Result, Context};

pub fn watch_files<P: AsRef<Path>>(
    path: P,
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
    ).wrap_err("Failed to create file watcher")?;

    watcher.watch(path.as_ref(), RecursiveMode::Recursive)
        .wrap_err("Failed to start watching files")?;

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