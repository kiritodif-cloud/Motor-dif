use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    #[serde(default = "default_title")]
    pub title: String,
    #[serde(default = "default_entry")]
    pub entry_scene: String,
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
}

fn default_title() -> String { "Motor-dif Game".to_string() }
fn default_entry() -> String { "assets/scene.json".to_string() }
fn default_width() -> u32 { 1280 }
fn default_height() -> u32 { 720 }

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            title: default_title(),
            entry_scene: default_entry(),
            width: default_width(),
            height: default_height(),
        }
    }
}

pub fn load_game_config_from_dir(dir: &Path) -> Option<GameConfig> {
    let p = dir.join("game.json");
    if p.exists() {
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(cfg) = serde_json::from_str::<GameConfig>(&content) {
                return Some(cfg);
            }
        }
    }
    None
}

pub fn check_auto_boot_game() -> Option<GameConfig> {
    // 1. Check current working directory
    if let Some(cfg) = load_game_config_from_dir(Path::new(".")) {
        return Some(cfg);
    }
    // 2. Check next to executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            if let Some(cfg) = load_game_config_from_dir(parent) {
                return Some(cfg);
            }
        }
    }
    None
}

pub fn export_game(
    game_name: &str,
    entry_scene: &str,
    output_dir: Option<&str>,
) -> Result<PathBuf, String> {
    let clean_name = game_name.trim();
    if clean_name.is_empty() {
        return Err("El nombre del juego no puede estar vacío".to_string());
    }

    let out_dir = match output_dir {
        Some(dir) => PathBuf::from(dir).join(clean_name),
        None => PathBuf::from("dist").join(clean_name),
    };

    fs::create_dir_all(&out_dir)
        .map_err(|e| format!("Error al crear directorio de salida '{:?}': {}", out_dir, e))?;

    // 1. Copy executable
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Error al obtener la ruta del ejecutable actual: {}", e))?;

    let exe_filename = if cfg!(windows) && !clean_name.to_lowercase().ends_with(".exe") {
        format!("{}.exe", clean_name)
    } else {
        clean_name.to_string()
    };
    let target_exe = out_dir.join(exe_filename);

    fs::copy(&current_exe, &target_exe)
        .map_err(|e| format!("Error al copiar el ejecutable a '{:?}': {}", target_exe, e))?;

    // 2. Copy assets/ folder recursively
    let src_assets = Path::new("assets");
    let dest_assets = out_dir.join("assets");
    if src_assets.exists() {
        copy_dir_recursive(src_assets, &dest_assets)
            .map_err(|e| format!("Error al copiar assets: {}", e))?;
    }

    // 3. Generate game.json
    let game_config = GameConfig {
        title: clean_name.to_string(),
        entry_scene: entry_scene.trim().to_string(),
        width: 1280,
        height: 720,
    };
    let json_content = serde_json::to_string_pretty(&game_config)
        .map_err(|e| format!("Error serializando game.json: {}", e))?;
    let game_json_path = out_dir.join("game.json");
    fs::write(&game_json_path, json_content)
        .map_err(|e| format!("Error al escribir '{:?}': {}", game_json_path, e))?;

    Ok(out_dir)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let entry_path = entry.path();
        let file_name = entry.file_name();

        let name_str = file_name.to_string_lossy();
        if name_str.starts_with('.') || name_str.ends_with(".tmp") || name_str.ends_with(".bak") {
            continue;
        }

        let dest_path = dst.join(file_name);
        if entry_path.is_dir() {
            copy_dir_recursive(&entry_path, &dest_path)?;
        } else {
            fs::copy(&entry_path, &dest_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_config_serialization() {
        let cfg = GameConfig {
            title: "Test Game".to_string(),
            entry_scene: "assets/cosmos_scene.json".to_string(),
            width: 1920,
            height: 1080,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let decoded: GameConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.title, "Test Game");
        assert_eq!(decoded.entry_scene, "assets/cosmos_scene.json");
        assert_eq!(decoded.width, 1920);
        assert_eq!(decoded.height, 1080);
    }
}
