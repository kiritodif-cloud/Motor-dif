use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
pub struct MouseInfo {
    pub screen_pos: [f32; 2],
    pub screen_size: [f32; 2],
    pub buttons_down: [bool; 3], // 0: Left, 1: Right, 2: Middle
    pub buttons_just_pressed: [bool; 3],
}

impl Default for MouseInfo {
    fn default() -> Self {
        Self {
            screen_pos: [0.0, 0.0],
            screen_size: [1280.0, 720.0],
            buttons_down: [false; 3],
            buttons_just_pressed: [false; 3],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputMap {
    pub actions: HashMap<String, Vec<String>>,
}

impl Default for InputMap {
    fn default() -> Self {
        let mut actions = HashMap::new();
        actions.insert("move_up".to_string(), vec!["W".to_string(), "ARROWUP".to_string()]);
        actions.insert("move_down".to_string(), vec!["S".to_string(), "ARROWDOWN".to_string()]);
        actions.insert("move_left".to_string(), vec!["A".to_string(), "ARROWLEFT".to_string()]);
        actions.insert("move_right".to_string(), vec!["D".to_string(), "ARROWRIGHT".to_string()]);
        actions.insert("jump".to_string(), vec!["SPACE".to_string()]);
        actions.insert("fire".to_string(), vec!["MOUSELEFT".to_string(), "F".to_string(), "E".to_string()]);
        actions.insert("alt_fire".to_string(), vec!["MOUSERIGHT".to_string(), "C".to_string()]);
        actions.insert("interact".to_string(), vec!["E".to_string()]);
        Self { actions }
    }
}

impl InputMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_or_default(path: &Path) -> Self {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(map) = serde_json::from_str::<InputMap>(&content) {
                    return map;
                }
            }
        }
        let def = Self::default();
        let _ = def.save_to_file(path);
        def
    }

    pub fn save_to_file(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, json)
    }

    pub fn bind(&mut self, action: &str, input: &str) {
        let list = self.actions.entry(action.to_string()).or_insert_with(Vec::new);
        let clean = input.trim().to_uppercase();
        if !list.iter().any(|k| k.to_uppercase() == clean) {
            list.push(input.trim().to_string());
        }
    }

    pub fn is_input_down(
        input: &str,
        pressed_keys: &HashSet<String>,
        mouse_buttons: &[bool; 3],
    ) -> bool {
        let clean = input.trim().to_uppercase();
        match clean.as_str() {
            "MOUSELEFT" | "MOUSE_LEFT" | "LMB" => mouse_buttons[0],
            "MOUSERIGHT" | "MOUSE_RIGHT" | "RMB" => mouse_buttons[1],
            "MOUSEMIDDLE" | "MOUSE_MIDDLE" | "MMB" => mouse_buttons[2],
            _ => crate::scripting::check_key_match(pressed_keys, &clean),
        }
    }

    pub fn is_input_just_pressed(
        input: &str,
        just_pressed_keys: &HashSet<String>,
        just_pressed_mouse: &[bool; 3],
    ) -> bool {
        let clean = input.trim().to_uppercase();
        match clean.as_str() {
            "MOUSELEFT" | "MOUSE_LEFT" | "LMB" => just_pressed_mouse[0],
            "MOUSERIGHT" | "MOUSE_RIGHT" | "RMB" => just_pressed_mouse[1],
            "MOUSEMIDDLE" | "MOUSE_MIDDLE" | "MMB" => just_pressed_mouse[2],
            _ => crate::scripting::check_key_match(just_pressed_keys, &clean),
        }
    }

    pub fn is_action_down(
        &self,
        action: &str,
        pressed_keys: &HashSet<String>,
        mouse_buttons: &[bool; 3],
    ) -> bool {
        if let Some(inputs) = self.actions.get(action) {
            inputs.iter().any(|inp| Self::is_input_down(inp, pressed_keys, mouse_buttons))
        } else {
            false
        }
    }

    pub fn is_action_just_pressed(
        &self,
        action: &str,
        just_pressed_keys: &HashSet<String>,
        just_pressed_mouse: &[bool; 3],
    ) -> bool {
        if let Some(inputs) = self.actions.get(action) {
            inputs.iter().any(|inp| Self::is_input_just_pressed(inp, just_pressed_keys, just_pressed_mouse))
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_map_bindings_and_actions() {
        let mut map = InputMap::new();
        let mut pressed_keys = HashSet::new();
        let mut mouse_down = [false; 3];

        // Initially move_up is W or ArrowUp
        assert!(!map.is_action_down("move_up", &pressed_keys, &mouse_down));

        pressed_keys.insert("W".to_string());
        assert!(map.is_action_down("move_up", &pressed_keys, &mouse_down));

        // Test fire with MouseLeft
        assert!(!map.is_action_down("fire", &pressed_keys, &mouse_down));
        mouse_down[0] = true;
        assert!(map.is_action_down("fire", &pressed_keys, &mouse_down));

        // Test custom binding
        map.bind("dash", "Shift");
        assert!(!map.is_action_down("dash", &pressed_keys, &mouse_down));
        pressed_keys.insert("SHIFT".to_string());
        assert!(map.is_action_down("dash", &pressed_keys, &mouse_down));
    }

    #[test]
    fn test_input_map_save_load() {
        let path = Path::new("target/test_input_map.json");
        let mut map = InputMap::new();
        map.bind("special_move", "Q");
        map.save_to_file(path).unwrap();

        let loaded = InputMap::load_or_default(path);
        assert!(loaded.actions.contains_key("special_move"));
        let _ = fs::remove_file(path);
    }
}
