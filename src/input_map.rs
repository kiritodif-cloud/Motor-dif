use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
pub struct MouseInfo {
    pub screen_pos: [f32; 2],
    pub delta: [f32; 2],
    pub screen_size: [f32; 2],
    pub buttons_down: [bool; 3], // 0: Left, 1: Right, 2: Middle
    pub buttons_just_pressed: [bool; 3],
}

impl Default for MouseInfo {
    fn default() -> Self {
        Self {
            screen_pos: [0.0, 0.0],
            delta: [0.0, 0.0],
            screen_size: [1280.0, 720.0],
            buttons_down: [false; 3],
            buttons_just_pressed: [false; 3],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GamepadState {
    pub left_stick: [f32; 2],  // x, y (-1.0 .. 1.0)
    pub right_stick: [f32; 2], // x, y (-1.0 .. 1.0)
    pub triggers: [f32; 2],    // left, right (0.0 .. 1.0)
    pub buttons_down: u32,
    pub buttons_just_pressed: u32,
    pub deadzone: f32,
}

impl Default for GamepadState {
    fn default() -> Self {
        Self {
            left_stick: [0.0, 0.0],
            right_stick: [0.0, 0.0],
            triggers: [0.0, 0.0],
            buttons_down: 0,
            buttons_just_pressed: 0,
            deadzone: 0.15,
        }
    }
}

pub struct GamepadBackend {
    pub enabled: bool,
}

impl GamepadBackend {
    pub fn init() -> Self {
        if cfg!(test)
            || std::env::var("CI").is_ok()
            || std::env::var("GITHUB_ACTIONS").is_ok()
        {
            return Self { enabled: false };
        }
        Self { enabled: true }
    }

    pub fn poll(&mut self, state: &mut GamepadState) {
        if !self.enabled {
            state.buttons_just_pressed = 0;
        }
    }
}

pub struct GamepadButton;
impl GamepadButton {
    pub const SOUTH: u32 = 1 << 0; // A / Cross
    pub const EAST: u32 = 1 << 1;  // B / Circle
    pub const WEST: u32 = 1 << 2;  // X / Square
    pub const NORTH: u32 = 1 << 3; // Y / Triangle
    pub const DPAD_UP: u32 = 1 << 4;
    pub const DPAD_DOWN: u32 = 1 << 5;
    pub const DPAD_LEFT: u32 = 1 << 6;
    pub const DPAD_RIGHT: u32 = 1 << 7;
    pub const L1: u32 = 1 << 8;
    pub const R1: u32 = 1 << 9;
    pub const L2: u32 = 1 << 10;
    pub const R2: u32 = 1 << 11;
    pub const L3: u32 = 1 << 12;
    pub const R3: u32 = 1 << 13;
    pub const START: u32 = 1 << 14;
    pub const SELECT: u32 = 1 << 15;
}

impl GamepadState {
    pub fn is_button_down(&self, button_name: &str) -> bool {
        let mask = Self::button_mask(button_name);
        (self.buttons_down & mask) != 0
    }

    pub fn is_button_just_pressed(&self, button_name: &str) -> bool {
        let mask = Self::button_mask(button_name);
        (self.buttons_just_pressed & mask) != 0
    }

    pub fn button_mask(name: &str) -> u32 {
        let clean = name.trim().to_uppercase();
        match clean.as_str() {
            "GAMEPAD_SOUTH" | "GAMEPAD_A" | "GAMEPAD_CROSS" | "PAD_A" => GamepadButton::SOUTH,
            "GAMEPAD_EAST" | "GAMEPAD_B" | "GAMEPAD_CIRCLE" | "PAD_B" => GamepadButton::EAST,
            "GAMEPAD_WEST" | "GAMEPAD_X" | "GAMEPAD_SQUARE" | "PAD_X" => GamepadButton::WEST,
            "GAMEPAD_NORTH" | "GAMEPAD_Y" | "GAMEPAD_TRIANGLE" | "PAD_Y" => GamepadButton::NORTH,
            "GAMEPAD_DPAD_UP" | "DPAD_UP" | "PAD_UP" => GamepadButton::DPAD_UP,
            "GAMEPAD_DPAD_DOWN" | "DPAD_DOWN" | "PAD_DOWN" => GamepadButton::DPAD_DOWN,
            "GAMEPAD_DPAD_LEFT" | "DPAD_LEFT" | "PAD_LEFT" => GamepadButton::DPAD_LEFT,
            "GAMEPAD_DPAD_RIGHT" | "DPAD_RIGHT" | "PAD_RIGHT" => GamepadButton::DPAD_RIGHT,
            "GAMEPAD_L1" | "GAMEPAD_LB" | "LB" | "L1" => GamepadButton::L1,
            "GAMEPAD_R1" | "GAMEPAD_RB" | "RB" | "R1" => GamepadButton::R1,
            "GAMEPAD_L2" | "GAMEPAD_LT" | "LT" | "L2" => GamepadButton::L2,
            "GAMEPAD_R2" | "GAMEPAD_RT" | "RT" | "R2" => GamepadButton::R2,
            "GAMEPAD_L3" | "L3" => GamepadButton::L3,
            "GAMEPAD_R3" | "R3" => GamepadButton::R3,
            "GAMEPAD_START" | "START" => GamepadButton::START,
            "GAMEPAD_SELECT" | "SELECT" | "BACK" => GamepadButton::SELECT,
            _ => 0,
        }
    }

    pub fn filter_deadzone(val: f32, deadzone: f32) -> f32 {
        if val.abs() < deadzone {
            0.0
        } else {
            let sign = val.signum();
            let scaled = (val.abs() - deadzone) / (1.0 - deadzone);
            (sign * scaled).clamp(-1.0, 1.0)
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
        // Standard snake_case
        actions.insert("move_up".to_string(), vec!["W".to_string(), "ARROWUP".to_string(), "GAMEPAD_DPAD_UP".to_string()]);
        actions.insert("move_down".to_string(), vec!["S".to_string(), "ARROWDOWN".to_string(), "GAMEPAD_DPAD_DOWN".to_string()]);
        actions.insert("move_left".to_string(), vec!["A".to_string(), "ARROWLEFT".to_string(), "GAMEPAD_DPAD_LEFT".to_string()]);
        actions.insert("move_right".to_string(), vec!["D".to_string(), "ARROWRIGHT".to_string(), "GAMEPAD_DPAD_RIGHT".to_string()]);
        actions.insert("jump".to_string(), vec!["SPACE".to_string(), "GAMEPAD_SOUTH".to_string()]);
        actions.insert("fire".to_string(), vec!["MOUSELEFT".to_string(), "F".to_string(), "GAMEPAD_R1".to_string()]);
        actions.insert("alt_fire".to_string(), vec!["MOUSERIGHT".to_string(), "C".to_string(), "GAMEPAD_L1".to_string()]);
        actions.insert("interact".to_string(), vec!["E".to_string(), "GAMEPAD_WEST".to_string()]);

        // Action-based PascalCase
        actions.insert("MoveForward".to_string(), vec!["W".to_string(), "ARROWUP".to_string(), "GAMEPAD_DPAD_UP".to_string()]);
        actions.insert("MoveBackward".to_string(), vec!["S".to_string(), "ARROWDOWN".to_string(), "GAMEPAD_DPAD_DOWN".to_string()]);
        actions.insert("MoveLeft".to_string(), vec!["A".to_string(), "ARROWLEFT".to_string(), "GAMEPAD_DPAD_LEFT".to_string()]);
        actions.insert("MoveRight".to_string(), vec!["D".to_string(), "ARROWRIGHT".to_string(), "GAMEPAD_DPAD_RIGHT".to_string()]);
        actions.insert("Jump".to_string(), vec!["SPACE".to_string(), "GAMEPAD_SOUTH".to_string()]);
        actions.insert("Fire".to_string(), vec!["MOUSELEFT".to_string(), "F".to_string(), "GAMEPAD_R1".to_string()]);
        actions.insert("AltFire".to_string(), vec!["MOUSERIGHT".to_string(), "C".to_string(), "GAMEPAD_L1".to_string()]);
        actions.insert("Interact".to_string(), vec!["E".to_string(), "GAMEPAD_WEST".to_string()]);

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
                if let Ok(mut map) = serde_json::from_str::<InputMap>(&content) {
                    let def = Self::default();
                    for (k, v) in def.actions {
                        map.actions.entry(k).or_insert(v);
                    }
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
        gamepad: &GamepadState,
    ) -> bool {
        let clean = input.trim().to_uppercase();
        if clean.starts_with("GAMEPAD_") || clean.starts_with("PAD_") {
            return gamepad.is_button_down(&clean);
        }
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
        gamepad: &GamepadState,
    ) -> bool {
        let clean = input.trim().to_uppercase();
        if clean.starts_with("GAMEPAD_") || clean.starts_with("PAD_") {
            return gamepad.is_button_just_pressed(&clean);
        }
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
        gamepad: &GamepadState,
    ) -> bool {
        if let Some(inputs) = self.actions.get(action) {
            if inputs.iter().any(|inp| Self::is_input_down(inp, pressed_keys, mouse_buttons, gamepad)) {
                return true;
            }
        }
        let clean = action.trim().to_lowercase().replace('-', "_");
        let fallback_key = match clean.as_str() {
            "moveforward" | "move_forward" => "move_up",
            "movebackward" | "move_backward" => "move_down",
            "moveleft" => "move_left",
            "moveright" => "move_right",
            _ => clean.as_str(),
        };
        for (k, inputs) in &self.actions {
            if k.eq_ignore_ascii_case(action) || k.eq_ignore_ascii_case(fallback_key) {
                if inputs.iter().any(|inp| Self::is_input_down(inp, pressed_keys, mouse_buttons, gamepad)) {
                    return true;
                }
            }
        }
        false
    }

    pub fn is_action_just_pressed(
        &self,
        action: &str,
        just_pressed_keys: &HashSet<String>,
        just_pressed_mouse: &[bool; 3],
        gamepad: &GamepadState,
    ) -> bool {
        if let Some(inputs) = self.actions.get(action) {
            if inputs.iter().any(|inp| Self::is_input_just_pressed(inp, just_pressed_keys, just_pressed_mouse, gamepad)) {
                return true;
            }
        }
        let clean = action.trim().to_lowercase().replace('-', "_");
        let fallback_key = match clean.as_str() {
            "moveforward" | "move_forward" => "move_up",
            "movebackward" | "move_backward" => "move_down",
            "moveleft" => "move_left",
            "moveright" => "move_right",
            _ => clean.as_str(),
        };
        for (k, inputs) in &self.actions {
            if k.eq_ignore_ascii_case(action) || k.eq_ignore_ascii_case(fallback_key) {
                if inputs.iter().any(|inp| Self::is_input_just_pressed(inp, just_pressed_keys, just_pressed_mouse, gamepad)) {
                    return true;
                }
            }
        }
        false
    }

    pub fn get_axis(
        &self,
        axis_name: &str,
        pressed_keys: &HashSet<String>,
        mouse: &MouseInfo,
        gamepad: &GamepadState,
    ) -> f32 {
        let clean = axis_name.trim().to_lowercase();
        match clean.as_str() {
            "horizontal" | "move_x" => {
                let right = self.is_action_down("MoveRight", pressed_keys, &mouse.buttons_down, gamepad)
                    || self.is_action_down("move_right", pressed_keys, &mouse.buttons_down, gamepad);
                let left = self.is_action_down("MoveLeft", pressed_keys, &mouse.buttons_down, gamepad)
                    || self.is_action_down("move_left", pressed_keys, &mouse.buttons_down, gamepad);
                let mut val = 0.0f32;
                if right { val += 1.0; }
                if left { val -= 1.0; }
                let pad_x = GamepadState::filter_deadzone(gamepad.left_stick[0], gamepad.deadzone);
                if pad_x.abs() > 0.01 {
                    val = pad_x;
                }
                val.clamp(-1.0, 1.0)
            }
            "vertical" | "move_y" => {
                let fwd = self.is_action_down("MoveForward", pressed_keys, &mouse.buttons_down, gamepad)
                    || self.is_action_down("move_up", pressed_keys, &mouse.buttons_down, gamepad);
                let back = self.is_action_down("MoveBackward", pressed_keys, &mouse.buttons_down, gamepad)
                    || self.is_action_down("move_down", pressed_keys, &mouse.buttons_down, gamepad);
                let mut val = 0.0f32;
                if fwd { val += 1.0; }
                if back { val -= 1.0; }
                let pad_y = GamepadState::filter_deadzone(gamepad.left_stick[1], gamepad.deadzone);
                if pad_y.abs() > 0.01 {
                    val = pad_y;
                }
                val.clamp(-1.0, 1.0)
            }
            "mouse_x" | "mousex" => mouse.delta[0],
            "mouse_y" | "mousey" => mouse.delta[1],
            "left_stick_x" | "leftstickx" => GamepadState::filter_deadzone(gamepad.left_stick[0], gamepad.deadzone),
            "left_stick_y" | "leftsticky" => GamepadState::filter_deadzone(gamepad.left_stick[1], gamepad.deadzone),
            "right_stick_x" | "rightstickx" => GamepadState::filter_deadzone(gamepad.right_stick[0], gamepad.deadzone),
            "right_stick_y" | "rightsticky" => GamepadState::filter_deadzone(gamepad.right_stick[1], gamepad.deadzone),
            "trigger_left" | "triggerleft" | "lt" => gamepad.triggers[0],
            "trigger_right" | "triggerright" | "rt" => gamepad.triggers[1],
            _ => 0.0,
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
        let mut gamepad = GamepadState::default();

        assert!(!map.is_action_down("MoveForward", &pressed_keys, &mouse_down, &gamepad));

        pressed_keys.insert("W".to_string());
        assert!(map.is_action_down("MoveForward", &pressed_keys, &mouse_down, &gamepad));

        // Test fire with MouseLeft
        assert!(!map.is_action_down("Fire", &pressed_keys, &mouse_down, &gamepad));
        mouse_down[0] = true;
        assert!(map.is_action_down("Fire", &pressed_keys, &mouse_down, &gamepad));

        // Test Gamepad action
        mouse_down[0] = false;
        assert!(!map.is_action_down("Jump", &pressed_keys, &mouse_down, &gamepad));
        gamepad.buttons_down |= GamepadButton::SOUTH;
        assert!(map.is_action_down("Jump", &pressed_keys, &mouse_down, &gamepad));

        // Test custom binding
        map.bind("dash", "Shift");
        assert!(!map.is_action_down("dash", &pressed_keys, &mouse_down, &gamepad));
        pressed_keys.insert("SHIFT".to_string());
        assert!(map.is_action_down("dash", &pressed_keys, &mouse_down, &gamepad));
    }

    #[test]
    fn test_input_map_axes_and_deadzone() {
        let map = InputMap::new();
        let mut pressed_keys = HashSet::new();
        let mouse = MouseInfo::default();
        let mut gamepad = GamepadState::default();

        // Horizontal axis via keyboard
        assert_eq!(map.get_axis("Horizontal", &pressed_keys, &mouse, &gamepad), 0.0);
        pressed_keys.insert("D".to_string());
        assert_eq!(map.get_axis("Horizontal", &pressed_keys, &mouse, &gamepad), 1.0);
        pressed_keys.remove("D");
        pressed_keys.insert("A".to_string());
        assert_eq!(map.get_axis("Horizontal", &pressed_keys, &mouse, &gamepad), -1.0);
        pressed_keys.clear();

        // Gamepad stick inside deadzone (0.15)
        gamepad.left_stick[0] = 0.10;
        assert_eq!(map.get_axis("Horizontal", &pressed_keys, &mouse, &gamepad), 0.0);

        // Gamepad stick outside deadzone
        gamepad.left_stick[0] = 0.50;
        let axis = map.get_axis("Horizontal", &pressed_keys, &mouse, &gamepad);
        assert!(axis > 0.35 && axis < 0.45);
    }

    #[test]
    fn test_input_map_save_load() {
        let path = Path::new("target/test_input_config.json");
        let mut map = InputMap::new();
        map.bind("special_move", "Q");
        map.save_to_file(path).unwrap();

        let loaded = InputMap::load_or_default(path);
        assert!(loaded.actions.contains_key("special_move"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_gamepad_backend_headless_safe() {
        let mut backend = GamepadBackend::init();
        let mut state = GamepadState::default();
        state.buttons_just_pressed = GamepadButton::SOUTH;
        backend.poll(&mut state);
        assert_eq!(state.buttons_just_pressed, 0);
    }
}
