/// File system watching for auto-reload
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use std::path::Path;
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use std::sync::mpsc::{channel, Receiver};

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct FileWatcher {
    watcher: RecommendedWatcher,
    receiver: Receiver<Result<Event, notify::Error>>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl FileWatcher {
    pub fn new() -> Result<Self, notify::Error> {
        let (tx, rx) = channel();

        let watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default(),
        )?;

        Ok(Self {
            watcher,
            receiver: rx,
        })
    }

    pub fn watch(&mut self, path: &str) -> Result<(), notify::Error> {
        self.watcher
            .watch(Path::new(path), RecursiveMode::Recursive)
    }

    pub fn unwatch(&mut self, path: &str) -> Result<(), notify::Error> {
        self.watcher.unwatch(Path::new(path))
    }

    pub fn check_events(&self) -> Vec<String> {
        let mut changed_files = Vec::new();

        // Non-blocking check for events
        while let Ok(res) = self.receiver.try_recv() {
            if let Ok(event) = res {
                match event.kind {
                    notify::EventKind::Modify(_) | notify::EventKind::Create(_) => {
                        for path in event.paths {
                            if let Some(path_str) = path.to_str() {
                                // Only report .wj files
                                if path_str.ends_with(".wj") {
                                    changed_files.push(path_str.to_string());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        changed_files
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for FileWatcher {
    fn default() -> Self {
        Self::new().expect("Failed to create file watcher")
    }
}
