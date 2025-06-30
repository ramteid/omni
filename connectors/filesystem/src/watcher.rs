use crate::models::FilesystemSource;
use anyhow::Result;
use notify::{Config, Event, EventKind, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;
use tracing::{debug, error, info, warn};

pub struct FilesystemWatcher {
    source: FilesystemSource,
    event_sender: tokio_mpsc::UnboundedSender<FilesystemEvent>,
}

#[derive(Debug, Clone)]
pub enum FilesystemEvent {
    FileCreated(PathBuf),
    FileModified(PathBuf),
    FileDeleted(PathBuf),
}

impl FilesystemWatcher {
    pub fn new(
        source: FilesystemSource,
        event_sender: tokio_mpsc::UnboundedSender<FilesystemEvent>,
    ) -> Self {
        Self {
            source,
            event_sender,
        }
    }

    pub async fn start_watching(&self) -> Result<()> {
        info!(
            "Starting filesystem watcher for source: {} at path: {}",
            self.source.name,
            self.source.base_path.display()
        );

        // Create a channel for the file watcher
        let (tx, rx) = mpsc::channel();
        let event_sender = self.event_sender.clone();
        let source = self.source.clone();

        // Spawn a blocking task to handle the file watcher
        let watcher_task = tokio::task::spawn_blocking(move || {
            // Use PollWatcher for better compatibility with network filesystems
            let config = Config::default()
                .with_poll_interval(Duration::from_secs(2))
                .with_compare_contents(true);

            let mut watcher = notify::PollWatcher::new(
                move |result: notify::Result<Event>| {
                    if let Err(e) = tx.send(result) {
                        error!("Failed to send file watcher event: {}", e);
                    }
                },
                config,
            )?;

            // Start watching the base path recursively
            watcher.watch(&source.base_path, RecursiveMode::Recursive)?;

            info!("File watcher started successfully");

            // Keep the watcher alive and process events
            for event_result in rx {
                match event_result {
                    Ok(event) => {
                        if let Err(e) = Self::process_file_event(&event, &source, &event_sender) {
                            error!("Failed to process file event: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("File watcher error: {}", e);
                    }
                }
            }

            Ok::<(), anyhow::Error>(())
        });

        // Wait for the watcher task to complete (which should be never in normal operation)
        match watcher_task.await {
            Ok(Ok(())) => {
                warn!("File watcher task completed unexpectedly");
            }
            Ok(Err(e)) => {
                error!("File watcher task failed: {}", e);
                return Err(e);
            }
            Err(e) => {
                error!("File watcher task panicked: {}", e);
                return Err(anyhow::anyhow!("File watcher task panicked: {}", e));
            }
        }

        Ok(())
    }

    fn process_file_event(
        event: &Event,
        source: &FilesystemSource,
        event_sender: &tokio_mpsc::UnboundedSender<FilesystemEvent>,
    ) -> Result<()> {
        debug!("Processing file event: {:?}", event);

        for path in &event.paths {
            // Check if this file should be included based on our filters
            if !source.should_include_file(path) {
                debug!("Skipping file event due to filters: {}", path.display());
                continue;
            }

            // Skip directories
            if path.is_dir() {
                continue;
            }

            let filesystem_event = match event.kind {
                EventKind::Create(_) => {
                    debug!("File created: {}", path.display());
                    FilesystemEvent::FileCreated(path.clone())
                }
                EventKind::Modify(_) => {
                    debug!("File modified: {}", path.display());
                    FilesystemEvent::FileModified(path.clone())
                }
                EventKind::Remove(_) => {
                    debug!("File deleted: {}", path.display());
                    FilesystemEvent::FileDeleted(path.clone())
                }
                _ => {
                    // Other event types we don't care about
                    continue;
                }
            };

            if let Err(e) = event_sender.send(filesystem_event) {
                error!("Failed to send filesystem event: {}", e);
            }
        }

        Ok(())
    }
}

pub struct FilesystemEventProcessor {
    event_receiver: tokio_mpsc::UnboundedReceiver<FilesystemEvent>,
}

impl FilesystemEventProcessor {
    pub fn new(event_receiver: tokio_mpsc::UnboundedReceiver<FilesystemEvent>) -> Self {
        Self { event_receiver }
    }

    pub async fn process_events(&mut self) -> Result<()> {
        info!("Starting filesystem event processor");

        while let Some(event) = self.event_receiver.recv().await {
            if let Err(e) = self.handle_event(event).await {
                error!("Failed to handle filesystem event: {}", e);
            }
        }

        info!("Filesystem event processor stopped");
        Ok(())
    }

    async fn handle_event(&self, event: FilesystemEvent) -> Result<()> {
        match event {
            FilesystemEvent::FileCreated(path) => {
                info!("Handling file creation: {}", path.display());
                // TODO: Trigger indexing for the new file
            }
            FilesystemEvent::FileModified(path) => {
                info!("Handling file modification: {}", path.display());
                // TODO: Trigger re-indexing for the modified file
            }
            FilesystemEvent::FileDeleted(path) => {
                info!("Handling file deletion: {}", path.display());
                // TODO: Remove the file from the index
            }
        }

        Ok(())
    }
}
