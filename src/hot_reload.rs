use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotReloadEvent {
    SceneChanged,
    ScriptChanged(PathBuf),
    PrefabChanged(PathBuf),
    Other(PathBuf),
}

pub struct HotReloadManager {
    _watcher: RecommendedWatcher,
    rx: Receiver<notify::Result<Event>>,
    pub last_reload_status: Option<String>,
    pub last_reload_error: Option<String>,
    pub ignore_scene_change_until: Option<std::time::Instant>,
}

impl HotReloadManager {
    pub fn new(watch_dir: impl AsRef<Path>) -> Result<Self, String> {
        let (tx, rx) = channel();

        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })
        .map_err(|e| format!("Failed to create filesystem watcher: {}", e))?;

        watcher
            .watch(watch_dir.as_ref(), RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch directory '{}': {}", watch_dir.as_ref().display(), e))?;

        Ok(Self {
            _watcher: watcher,
            rx,
            last_reload_status: Some("Watching assets/ directory in real-time".to_string()),
            last_reload_error: None,
            ignore_scene_change_until: None,
        })
    }

    pub fn fallback(err: String) -> Self {
        let (_tx, rx) = channel();
        let watcher = notify::recommended_watcher(|_| {}).expect("Failed to create dummy watcher");
        Self {
            _watcher: watcher,
            rx,
            last_reload_status: None,
            last_reload_error: Some(err),
            ignore_scene_change_until: None,
        }
    }

    pub fn suppress_scene_events(&mut self, duration: std::time::Duration) {
        self.ignore_scene_change_until = Some(std::time::Instant::now() + duration);
    }

    pub fn poll_events(&mut self) -> Vec<HotReloadEvent> {
        let mut events = Vec::new();
        let mut scene_changed = false;

        while let Ok(res) = self.rx.try_recv() {
            if let Ok(event) = res {
                for path in event.paths {
                    let file_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_default()
                        .to_lowercase();

                    let path_str = path.to_string_lossy().to_lowercase();
                    if file_name.ends_with(".prefab.json") || path_str.contains("prefabs") {
                        events.push(HotReloadEvent::PrefabChanged(path));
                    } else if file_name.ends_with(".json") && file_name != "input_map.json" && file_name != "save_data.json" && file_name != "game.json" && file_name != "game_config.json" {
                        scene_changed = true;
                    } else if file_name.ends_with(".lua") {
                        events.push(HotReloadEvent::ScriptChanged(path));
                    } else {
                        events.push(HotReloadEvent::Other(path));
                    }
                }
            }
        }

        if scene_changed {
            if let Some(until) = self.ignore_scene_change_until {
                if std::time::Instant::now() < until {
                    scene_changed = false;
                } else {
                    self.ignore_scene_change_until = None;
                }
            }
        }

        if scene_changed {
            events.insert(0, HotReloadEvent::SceneChanged);
        }

        events
    }
}
