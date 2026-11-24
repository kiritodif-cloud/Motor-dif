use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    #[serde(default = "default_title")]
    pub title: String,
    #[serde(default = "default_entry")]
    pub entry_scene: String,
    #[serde(default)]
    pub fullscreen: bool,
    #[serde(default = "default_width", rename = "window_width", alias = "width")]
    pub width: u32,
    #[serde(default = "default_height", rename = "window_height", alias = "height")]
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
            fullscreen: false,
            width: default_width(),
            height: default_height(),
        }
    }
}

pub fn load_game_config_from_dir(dir: &Path) -> Option<GameConfig> {
    for filename in &["game_config.json", "game.json"] {
        let p = dir.join(filename);
        if p.exists() {
            if let Ok(content) = fs::read_to_string(&p) {
                if let Ok(cfg) = serde_json::from_str::<GameConfig>(&content) {
                    return Some(cfg);
                }
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

use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub output_dir: PathBuf,
    pub total_bytes: u64,
    pub total_mb: String,
    pub asset_count: usize,
    pub exe_name: String,
}

pub fn find_assets_in_lua_code(code: &str) -> Vec<String> {
    let mut found = Vec::new();
    let bytes = code.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' || bytes[i] == b'\'' {
            let quote = bytes[i];
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != quote && bytes[i] != b'\n' {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == quote {
                if let Ok(slice) = std::str::from_utf8(&bytes[start..i]) {
                    if slice.starts_with("assets/") {
                        found.push(slice.to_string());
                    }
                }
            }
        }
        i += 1;
    }
    found
}

pub fn resolve_asset_path(path_str: &str) -> Option<PathBuf> {
    let p = PathBuf::from(path_str.trim());
    if p.is_file() {
        return Some(p);
    }
    let p_in_assets = PathBuf::from("assets").join(path_str.trim());
    if p_in_assets.is_file() {
        return Some(p_in_assets);
    }
    None
}

pub fn harvest_scene_dependencies(entry_scene_path: &Path) -> Result<(HashSet<PathBuf>, bool), String> {
    let mut collected: HashSet<PathBuf> = HashSet::new();
    let mut pending_scenes: Vec<PathBuf> = vec![entry_scene_path.to_path_buf()];
    let mut visited_scenes: HashSet<PathBuf> = HashSet::new();
    let mut has_voxels = false;

    while let Some(scene_p) = pending_scenes.pop() {
        if !visited_scenes.insert(scene_p.clone()) {
            continue;
        }
        if !scene_p.is_file() {
            continue;
        }
        collected.insert(scene_p.clone());

        // Parse SceneDef JSON
        let content = fs::read_to_string(&scene_p)
            .map_err(|e| format!("Error reading scene '{}': {}", scene_p.display(), e))?;
        let scene: crate::assets::SceneDef = match serde_json::from_str(&content) {
            Ok(s) => s,
            Err(e) => {
                // If it fails to parse as SceneDef, it might still be a raw JSON asset, continue
                eprintln!("[Exporter] Notice: '{}' is not a SceneDef: {}", scene_p.display(), e);
                continue;
            }
        };

        for entity in &scene.entities {
            let mesh_str = entity.mesh.trim();
            if mesh_str == "voxel" || mesh_str.starts_with("voxel") {
                has_voxels = true;
            }
            if let Some(mesh_p) = resolve_asset_path(mesh_str) {
                collected.insert(mesh_p.clone());
                // Sidecars: .heightmap.json and .voxels.json
                let sidecar_hm = mesh_p.with_extension("heightmap.json");
                if sidecar_hm.is_file() {
                    collected.insert(sidecar_hm);
                }
                let sidecar_vx = mesh_p.with_extension("voxels.json");
                if sidecar_vx.is_file() {
                    has_voxels = true;
                    collected.insert(sidecar_vx);
                }
                let sidecar_hm2 = PathBuf::from(format!("{}.heightmap.json", mesh_p.display()));
                if sidecar_hm2.is_file() {
                    collected.insert(sidecar_hm2);
                }
                let sidecar_vx2 = PathBuf::from(format!("{}.voxels.json", mesh_p.display()));
                if sidecar_vx2.is_file() {
                    has_voxels = true;
                    collected.insert(sidecar_vx2);
                }
            }

            if let Some(tex) = &entity.texture {
                if let Some(tex_p) = resolve_asset_path(tex) {
                    collected.insert(tex_p);
                }
            }

            if let Some(script_str) = &entity.script {
                if let Some(script_p) = resolve_asset_path(script_str) {
                    collected.insert(script_p.clone());
                    if let Ok(lua_code) = fs::read_to_string(&script_p) {
                        if lua_code.contains("voxel_") || lua_code.contains("\"voxel\"") {
                            has_voxels = true;
                        }
                        for asset_str in find_assets_in_lua_code(&lua_code) {
                            if let Some(asset_p) = resolve_asset_path(&asset_str) {
                                collected.insert(asset_p.clone());
                                if asset_str.ends_with(".json") {
                                    pending_scenes.push(asset_p);
                                } else if asset_str.ends_with(".glb") {
                                    let sc_hm = asset_p.with_extension("heightmap.json");
                                    if sc_hm.is_file() { collected.insert(sc_hm); }
                                    let sc_vx = asset_p.with_extension("voxels.json");
                                    if sc_vx.is_file() { has_voxels = true; collected.insert(sc_vx); }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if has_voxels {
        let atlas = PathBuf::from("assets/textures/voxel_atlas.png");
        if atlas.is_file() {
            collected.insert(atlas);
        }
    }

    Ok((collected, has_voxels))
}

pub fn export_game(
    game_name: &str,
    entry_scene: &str,
    output_dir: Option<&str>,
    fullscreen: bool,
) -> Result<ExportResult, String> {
    let clean_name = game_name.trim();
    if clean_name.is_empty() {
        return Err("El nombre del juego no puede estar vacío".to_string());
    }

    let out_dir = match output_dir {
        Some(dir) => {
            let p = PathBuf::from(dir.trim());
            let already_ends_with_name = p
                .file_name()
                .and_then(|f| f.to_str())
                .map(|f| f.eq_ignore_ascii_case(clean_name))
                .unwrap_or(false);
            if already_ends_with_name {
                p
            } else {
                p.join(clean_name)
            }
        }
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
    let target_exe = out_dir.join(&exe_filename);

    fs::copy(&current_exe, &target_exe)
        .map_err(|e| format!("Error al copiar el ejecutable a '{:?}': {}", target_exe, e))?;

    let mut total_bytes = fs::metadata(&target_exe).map(|m| m.len()).unwrap_or(0);

    // 2. Harvest asset dependencies
    let entry_p = resolve_asset_path(entry_scene)
        .unwrap_or_else(|| PathBuf::from(entry_scene.trim()));

    let (harvested_files, _has_voxels) = harvest_scene_dependencies(&entry_p)?;

    let mut asset_count = 0;
    for file in &harvested_files {
        let dest = out_dir.join(file);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Error creating dir '{:?}': {}", parent, e))?;
        }
        if let Ok(bytes) = fs::copy(file, &dest) {
            total_bytes += bytes;
            asset_count += 1;
        }
    }

    // Fallback: If 0 assets harvested, ensure entry scene is at least copied
    if asset_count == 0 && entry_p.is_file() {
        let dest = out_dir.join(&entry_p);
        if let Some(parent) = dest.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(b) = fs::copy(&entry_p, &dest) {
            total_bytes += b;
            asset_count += 1;
        }
    }

    // 3. Generate game_config.json
    let game_config = GameConfig {
        title: clean_name.to_string(),
        entry_scene: entry_scene.trim().to_string(),
        fullscreen,
        width: 1280,
        height: 720,
    };
    let json_content = serde_json::to_string_pretty(&game_config)
        .map_err(|e| format!("Error serializando game_config.json: {}", e))?;
    let game_config_path = out_dir.join("game_config.json");
    fs::write(&game_config_path, &json_content)
        .map_err(|e| format!("Error al escribir '{:?}': {}", game_config_path, e))?;
    total_bytes += json_content.len() as u64;

    // Write game.json for legacy compatibility
    let game_json_path = out_dir.join("game.json");
    let _ = fs::write(&game_json_path, &json_content);

    let total_mb = format!("{:.2} MB", total_bytes as f64 / (1024.0 * 1024.0));

    Ok(ExportResult {
        output_dir: out_dir,
        total_bytes,
        total_mb,
        asset_count,
        exe_name: exe_filename,
    })
}

#[allow(dead_code)]
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
            fullscreen: true,
            width: 1920,
            height: 1080,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(json.contains("\"window_width\":1920"));
        assert!(json.contains("\"fullscreen\":true"));
        let decoded: GameConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.title, "Test Game");
        assert_eq!(decoded.entry_scene, "assets/cosmos_scene.json");
        assert_eq!(decoded.fullscreen, true);
        assert_eq!(decoded.width, 1920);
        assert_eq!(decoded.height, 1080);

        // Test backward compatibility with legacy "width"/"height" json
        let legacy_json = r#"{"title":"Legacy","entry_scene":"assets/scene.json","width":800,"height":600}"#;
        let legacy_cfg: GameConfig = serde_json::from_str(legacy_json).unwrap();
        assert_eq!(legacy_cfg.title, "Legacy");
        assert_eq!(legacy_cfg.fullscreen, false);
        assert_eq!(legacy_cfg.width, 800);
        assert_eq!(legacy_cfg.height, 600);
    }

    #[test]
    fn test_export_output_dir_resolution() {
        let resolve_dir = |dir: Option<&str>, name: &str| -> PathBuf {
            let clean_name = name.trim();
            match dir {
                Some(d) => {
                    let p = PathBuf::from(d.trim());
                    let already_ends = p
                        .file_name()
                        .and_then(|f| f.to_str())
                        .map(|f| f.eq_ignore_ascii_case(clean_name))
                        .unwrap_or(false);
                    if already_ends {
                        p
                    } else {
                        p.join(clean_name)
                    }
                }
                None => PathBuf::from("dist").join(clean_name),
            }
        };

        // When passing output_dir = "dist" and name = "mi_juego" -> "dist/mi_juego"
        assert_eq!(resolve_dir(Some("dist"), "mi_juego"), PathBuf::from("dist").join("mi_juego"));

        // When passing output_dir = "dist/mi_juego" and name = "mi_juego" -> "dist/mi_juego" (NO DUPLICATION)
        assert_eq!(resolve_dir(Some("dist/mi_juego"), "mi_juego"), PathBuf::from("dist/mi_juego"));

        // When passing output_dir = None -> "dist/mi_juego"
        assert_eq!(resolve_dir(None, "mi_juego"), PathBuf::from("dist").join("mi_juego"));
    }

    #[test]
    fn test_find_assets_in_lua_code() {
        let lua = r#"
            local s = "assets/models/player.glb"
            play_sound('assets/audio/laser.wav')
            load_scene("assets/dungeon_lvl2.json")
            local ignore = "hello world"
            local relative = "scripts/helper.lua"
        "#;
        let assets = find_assets_in_lua_code(lua);
        assert_eq!(assets.len(), 3);
        assert!(assets.contains(&"assets/models/player.glb".to_string()));
        assert!(assets.contains(&"assets/audio/laser.wav".to_string()));
        assert!(assets.contains(&"assets/dungeon_lvl2.json".to_string()));
    }

    #[test]
    fn test_harvest_scene_dependencies_mock() {
        let temp_dir = std::env::temp_dir().join("motor_dif_harvest_test");
        let _ = fs::remove_dir_all(&temp_dir);
        let assets_dir = temp_dir.join("assets");
        fs::create_dir_all(&assets_dir).unwrap();

        // 1. Create dummy files
        let dummy_mesh = assets_dir.join("arena.glb");
        fs::write(&dummy_mesh, b"glb_data").unwrap();
        let dummy_sidecar = assets_dir.join("arena.heightmap.json");
        fs::write(&dummy_sidecar, b"{}").unwrap();
        let dummy_script = assets_dir.join("script.lua");
        fs::write(&dummy_script, b"local a = 'assets/audio.wav'").unwrap();
        let dummy_audio = assets_dir.join("audio.wav");
        fs::write(&dummy_audio, b"wav_data").unwrap();
        let unused_file = assets_dir.join("unused.png");
        fs::write(&unused_file, b"png_data").unwrap();

        // 2. Create scene referencing dummy_mesh and dummy_script
        let scene_json = format!(r#"{{
            "camera": {{ "mode": "3d", "position": [0,0,0], "target": [0,0,0], "ortho_size": 10 }},
            "entities": [
                {{
                    "id": "e1",
                    "mesh": "{}",
                    "position": [0,0,0],
                    "color": [1,1,1,1],
                    "script": "{}"
                }}
            ]
        }}"#, dummy_mesh.to_str().unwrap().replace('\\', "/"), dummy_script.to_str().unwrap().replace('\\', "/"));

        let scene_file = assets_dir.join("test_scene.json");
        fs::write(&scene_file, scene_json).unwrap();

        let (harvested, _has_voxels) = harvest_scene_dependencies(&scene_file).unwrap();
        assert!(harvested.contains(&scene_file));
        assert!(harvested.contains(&dummy_mesh));
        assert!(harvested.contains(&dummy_sidecar));
        assert!(harvested.contains(&dummy_script));

        // Unused file must NOT be harvested
        assert!(!harvested.contains(&unused_file));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
