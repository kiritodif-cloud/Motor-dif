//! Static checks for engine Lua scripts.
//!
//! The linter is intentionally dependency-light and conservative: scripts are
//! scanned after stripping comments and string literals so documentation and
//! examples never trigger diagnostics.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LintDiagnostic {
    pub code: &'static str,
    pub severity: &'static str,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub hint: String,
}

#[derive(Debug, Clone, Copy)]
struct CodeChar {
    ch: char,
    line: usize,
    column: usize,
}

/// Returns only the characters that are actual Lua code, preserving line and
/// column information. Comments and short/long strings are replaced by spaces.
fn code_chars(source: &str) -> Vec<CodeChar> {
    // Build position metadata first so the scanning pass can use indexed
    // lookahead instead of a clonable iterator, and never slices by byte.
    let mut raw = Vec::with_capacity(source.chars().count());
    let mut line = 1usize;
    let mut column = 1usize;
    for ch in source.chars() {
        raw.push(CodeChar { ch, line, column });
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    enum Mode {
        Code,
        Short(char),
        Long,
        LineComment,
        LongComment,
    }

    let mut out = Vec::with_capacity(raw.len());
    let mut mode = Mode::Code;
    let mut long_level = 0usize;
    let mut i = 0usize;

    while i < raw.len() {
        let CodeChar { ch, line, column } = raw[i];
        let space = CodeChar {
            ch: ' ',
            line,
            column,
        };
        match mode {
            Mode::Code => match ch {
                '-' if i + 1 < raw.len() && raw[i + 1].ch == '-' => {
                    out.push(space);
                    out.push(CodeChar {
                        ch: ' ',
                        line,
                        column: column + 1,
                    });
                    i += 2;
                    let mut level = 0usize;
                    if i < raw.len() && raw[i].ch == '[' {
                        let mut eq = 0usize;
                        let mut j = i + 1;
                        while j < raw.len() {
                            if raw[j].ch == '=' {
                                eq += 1;
                                j += 1;
                            } else if raw[j].ch == '[' {
                                level = eq + 1;
                                break;
                            } else {
                                break;
                            }
                        }
                    }
                    if level > 0 {
                        long_level = level;
                        mode = Mode::LongComment;
                    } else {
                        mode = Mode::LineComment;
                    }
                }
                '"' | '\'' => {
                    out.push(space);
                    mode = Mode::Short(ch);
                    i += 1;
                }
                '[' => {
                    let mut eq = 0usize;
                    let mut j = i + 1;
                    let mut found = false;
                    while j < raw.len() {
                        if raw[j].ch == '=' {
                            eq += 1;
                            j += 1;
                        } else if raw[j].ch == '[' {
                            long_level = eq;
                            found = true;
                            break;
                        } else {
                            break;
                        }
                    }
                    if found {
                        out.push(space);
                        mode = Mode::Long;
                    } else {
                        out.push(raw[i]);
                    }
                    i += 1;
                }
                _ => {
                    out.push(raw[i]);
                    i += 1;
                }
            },
            Mode::Short(quote) => {
                out.push(space);
                if ch == '\\' && i + 1 < raw.len() {
                    out.push(CodeChar {
                        ch: ' ',
                        line: raw[i + 1].line,
                        column: raw[i + 1].column,
                    });
                    i += 2;
                } else {
                    if ch == quote {
                        mode = Mode::Code;
                    }
                    i += 1;
                }
            }
            Mode::LineComment => {
                out.push(space);
                if ch == '\n' {
                    mode = Mode::Code;
                }
                i += 1;
            }
            Mode::LongComment | Mode::Long => {
                out.push(space);
                i += 1;
                if ch == ']' {
                    let mut eq = 0usize;
                    while i < raw.len() && raw[i].ch == '=' {
                        out.push(CodeChar {
                            ch: ' ',
                            line: raw[i].line,
                            column: raw[i].column,
                        });
                        eq += 1;
                        i += 1;
                    }
                    if i < raw.len() && raw[i].ch == ']' && eq == long_level {
                        out.push(CodeChar {
                            ch: ' ',
                            line: raw[i].line,
                            column: raw[i].column,
                        });
                        i += 1;
                        mode = Mode::Code;
                    }
                }
            }
        }
    }
    out
}

fn is_ident(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

struct CodeText {
    text: String,
    map: Vec<(usize, usize)>,
}

impl CodeText {
    fn new(chars: &[CodeChar]) -> Self {
        let mut text = String::with_capacity(chars.len());
        let mut map = Vec::with_capacity(chars.len());
        for c in chars {
            text.push(c.ch);
            map.push((c.line, c.column));
        }
        Self { text, map }
    }

    fn slice_from(&self, byte: usize) -> &str {
        let mut start = byte.min(self.text.len());
        while !self.text.is_char_boundary(start) {
            start += 1;
        }
        &self.text[start..]
    }

    fn slice_range(&self, start: usize, end: usize) -> &str {
        let mut start = start.min(self.text.len());
        while !self.text.is_char_boundary(start) {
            start += 1;
        }
        let mut end = end.min(self.text.len()).max(start);
        while !self.text.is_char_boundary(end) {
            end += 1;
        }
        &self.text[start..end]
    }

    fn pos(&self, byte: usize) -> (usize, usize) {
        let idx = self
            .slice_range(0, byte)
            .chars()
            .count()
            .min(self.map.len().saturating_sub(1));
        self.map[idx]
    }

    fn find_ident(&self, name: &str) -> Vec<usize> {
        self.find_name(name, false)
    }

    fn find_prefix(&self, name: &str) -> Vec<usize> {
        self.find_name(name, true)
    }

    fn find_name(&self, name: &str, prefix: bool) -> Vec<usize> {
        let mut hits = Vec::new();
        let bytes = self.text.as_bytes();
        let needle = name.as_bytes();
        if needle.is_empty() {
            return hits;
        }
        let mut i = 0;
        while i + needle.len() <= bytes.len() {
            if &bytes[i..i + needle.len()] == needle {
                let before = i == 0 || !is_ident(bytes[i - 1] as char);
                let after = prefix
                    || i + needle.len() == bytes.len()
                    || !is_ident(bytes[i + needle.len()] as char);
                if before && after {
                    hits.push(i);
                    i += needle.len();
                    continue;
                }
            }
            i += 1;
        }
        hits
    }

    fn count_substring(&self, needle: &str) -> usize {
        self.text
            .as_bytes()
            .windows(needle.len())
            .filter(|w| *w == needle.as_bytes())
            .count()
    }
}

fn diag(
    code: &'static str,
    severity: &'static str,
    pos: (usize, usize),
    message: &str,
    hint: &str,
) -> LintDiagnostic {
    LintDiagnostic {
        code,
        severity,
        line: pos.0,
        column: pos.1,
        message: message.to_string(),
        hint: hint.to_string(),
    }
}

/// Lint the given Lua source for Motor-dif engine API contract violations.
pub fn lint_lua_source(source: &str) -> Vec<LintDiagnostic> {
    let chars = code_chars(source);
    let code = CodeText::new(&chars);
    let mut out = Vec::new();

    for pos in code.find_ident("set_hud_text") {
        out.push(diag(
            "LUA001_NO_SET_HUD_TEXT",
            "error",
            code.pos(pos),
            "set_hud_text is forbidden for game HUDs",
            "Use ui_panel, ui_bar, ui_label and ui_button from lua_ui instead",
        ));
    }

    for pos in code.find_ident("on_update") {
        let rest = code.slice_from(pos);
        let after_paren = match rest.find('(') {
            Some(i) if rest[..i].trim() == "on_update" => &rest[i + 1..],
            _ => continue,
        };
        let params: String = after_paren.chars().take_while(|c| *c != ')').collect();
        let first = params.split(',').next().unwrap_or("").trim();
        if first == "dt" || first.is_empty() {
            out.push(diag(
                "LUA002_ON_UPDATE_SIGNATURE",
                "error",
                code.pos(pos),
                "on_update must receive the entity table as its first argument",
                "Use `function on_update(entity, dt)` and the init idiom from AGENT_GUIDE.md",
            ));
        }
    }

    for name in [
        "get_position",
        "set_position",
        "get_rotation",
        "set_rotation",
    ] {
        for pos in code.find_ident(name) {
            out.push(diag(
                "LUA003_LEGACY_TRANSFORM_GLOBAL",
                "error",
                code.pos(pos),
                &format!("`{}` is not registered by the engine", name),
                "Use entity.x/y/z and entity.rx/ry/rz (radians)",
            ));
        }
    }

    // `math.deg(...)` assigned to an entity rotation field.
    let fields = ["entity.rx", "entity.ry", "entity.rz"];
    for pos in code.find_ident("math.deg") {
        let tail = code.slice_from(pos);
        if tail.len() > 4000 {
            continue;
        }
        if fields.iter().any(|f| tail.contains(f)) {
            out.push(diag(
                "LUA004_DEGREE_ROTATION",
                "error",
                code.pos(pos),
                "math.deg used near an entity rotation assignment",
                "Entity rotations are radians; remove math.deg and use radians directly",
            ));
        }
    }

    // Reset functions must clear UI, timers and events.
    let reset_names = ["reset", "restart"];
    let cleanup = ["ui_clear", "clear_timers", "clear_events"];
    for name in reset_names {
        for pos in code.find_prefix(name) {
            let tail = code.slice_from(pos);
            let head = code.slice_range(pos.saturating_sub(2000), pos);
            let is_function = head
                .rfind("function")
                .map(|i| pos - i < 80)
                .unwrap_or(false)
                || tail
                    .find('(')
                    .map(|i| i < 80 && tail[..i].contains('_'))
                    .unwrap_or(false);
            if !is_function {
                continue;
            }
            let end = tail.find("\nend").unwrap_or(tail.len().min(6000));
            let body = &tail[..end];
            if !cleanup.iter().all(|c| body.contains(c)) {
                out.push(diag(
                    "LUA005_RESET_CLEANUP_MISSING",
                    "warning",
                    code.pos(pos),
                    &format!("`{}` function does not call the full cleanup trio", name),
                    "Call ui_clear(), clear_timers() and clear_events() when resetting state",
                ));
            }
        }
    }

    // Init idiom: on_init must be called from on_update via a guard.
    if code.find_ident("on_init").len() == 1
        && code.find_ident("on_update").len() == 1
        && code.count_substring("on_init(") <= 1
    {
        let pos = code.find_ident("on_init")[0];
        out.push(diag(
            "LUA006_INIT_IDIOM_MISSING",
            "warning",
            code.pos(pos),
            "on_init is defined but never called",
            "Add `local initialized=false` and call `on_init(entity)` from on_update on the first frame",
        ));
    }

    for pos in code.find_ident("get_mouse_delta") {
        let head = code.slice_range(pos.saturating_sub(100), pos);
        if head.contains("local ") && head.contains(',') {
            out.push(diag(
                "LUA007_MOUSE_DELTA_SHAPE",
                "error",
                code.pos(pos),
                "get_mouse_delta returns a table, not two values",
                "Use `local d = get_mouse_delta(); d.dx`, `d.dy`",
            ));
        }
    }

    for pos in code.find_ident("block_y") {
        out.push(diag(
            "LUA008_VOXEL_RAYCAST_FIELD",
            "error",
            code.pos(pos),
            "voxel_raycast does not return `block_y`",
            "Use the `y` field of the raycast result table",
        ));
    }

    for pos in code.find_ident("is_mouse_button_just_pressed") {
        let tail = code.slice_range(pos, pos.saturating_add(100));
        if tail.contains("(1)") {
            out.push(diag(
                "LUA009_MOUSE_BUTTON_HINT",
                "info",
                code.pos(pos),
                "Mouse button 1 is the right button",
                "Use 0 or \"left\" for the primary button",
            ));
        }
    }

    out.sort_by_key(|d| (d.line, d.column));
    out
}

/// Lint a Lua file from disk.
pub fn lint_lua_file(path: &str) -> Result<Vec<LintDiagnostic>, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("Error reading Lua file '{}': {}", path, e))?;
    Ok(lint_lua_source(&source))
}

/// Human-readable diagnostic list for MCP responses.
pub fn format_diagnostics(path: &str, diags: &[LintDiagnostic]) -> String {
    if diags.is_empty() {
        return format!("{}: no lint issues", path);
    }
    let mut lines = Vec::with_capacity(diags.len() + 1);
    lines.push(format!("{}: {} lint issue(s)", path, diags.len()));
    for d in diags {
        lines.push(format!(
            "  {}:{} {} [{}] {} ({})",
            d.line, d.column, d.severity, d.code, d.message, d.hint
        ));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_forbidden_hud_and_ignores_comments_and_strings() {
        let src = "-- set_hud_text('x')\nlocal s = 'set_hud_text'\nset_hud_text('x')\n";
        let d = lint_lua_source(src);
        assert_eq!(d.len(), 1, "{:?}", d);
        assert_eq!(d[0].code, "LUA001_NO_SET_HUD_TEXT");
        assert_eq!(d[0].line, 3);
    }

    #[test]
    fn detects_bad_update_signature() {
        let d = lint_lua_source("function on_update(dt) end\n");
        assert!(d.iter().any(|x| x.code == "LUA002_ON_UPDATE_SIGNATURE"));
        assert!(lint_lua_source("function on_update(entity, dt) end\n").is_empty());
    }

    #[test]
    fn detects_legacy_transform_globals() {
        let d = lint_lua_source("local x,y,z = get_position()\nset_position(1,2,3)\n");
        assert_eq!(
            d.iter()
                .filter(|x| x.code == "LUA003_LEGACY_TRANSFORM_GLOBAL")
                .count(),
            2
        );
    }

    #[test]
    fn detects_degrees_assigned_to_rotation() {
        let d = lint_lua_source("local yaw = math.deg(1.0)\nentity.ry = yaw\n");
        assert!(d.iter().any(|x| x.code == "LUA004_DEGREE_ROTATION"));
    }

    #[test]
    fn detects_missing_reset_cleanup() {
        let d = lint_lua_source("function reset_game()\n  score = 0\nend\n");
        assert!(d.iter().any(|x| x.code == "LUA005_RESET_CLEANUP_MISSING"));
        let ok = lint_lua_source(
            "function reset_game()\n  ui_clear()\n  clear_timers()\n  clear_events()\nend\n",
        );
        assert!(!ok.iter().any(|x| x.code == "LUA005_RESET_CLEANUP_MISSING"));
    }

    #[test]
    fn detects_unused_init_and_bad_mouse_delta() {
        let d =
            lint_lua_source("function on_init(entity) end\nfunction on_update(entity, dt) end\n");
        assert!(d.iter().any(|x| x.code == "LUA006_INIT_IDIOM_MISSING"));
        let d = lint_lua_source("local dx, dy = get_mouse_delta()\n");
        assert!(d.iter().any(|x| x.code == "LUA007_MOUSE_DELTA_SHAPE"));
    }

    #[test]
    fn detects_block_y_and_mouse_button_hint() {
        let d =
            lint_lua_source("local y = hit.block_y\nif is_mouse_button_just_pressed(1) then end\n");
        assert!(d.iter().any(|x| x.code == "LUA008_VOXEL_RAYCAST_FIELD"));
        assert!(d.iter().any(|x| x.code == "LUA009_MOUSE_BUTTON_HINT"));
    }

    #[test]
    fn all_scene_scripts_exist_and_compile() {
        let mut scenes = 0usize;
        let mut scripts = 0usize;
        for entry in std::fs::read_dir("assets").expect("assets dir") {
            let entry = entry.expect("dir entry");
            let name = entry.file_name().to_string_lossy().to_string();
            if !crate::is_scene_path(&name) {
                continue;
            }
            let path = format!("assets/{}", name);
            let scene = crate::assets::load_scene_from_file(&path).expect("scene should parse");
            scenes += 1;
            for entity in &scene.entities {
                let Some(script) = entity.script.as_deref() else {
                    continue;
                };
                let source = std::fs::read_to_string(script)
                    .unwrap_or_else(|e| panic!("missing script '{}': {}", script, e));
                let lua = mlua::Lua::new();
                lua.load(&source)
                    .set_name(script)
                    .into_function()
                    .unwrap_or_else(|e| panic!("script '{}' does not compile: {}", script, e));
                scripts += 1;
            }
        }
        assert!(scenes >= 10, "expected scenes, found {}", scenes);
        assert!(scripts >= 10, "expected scripts, found {}", scripts);
    }

    #[test]
    fn handles_utf8_source_without_panicking() {
        let src = "-- café ☃\nlocal s = 'ñ'\nset_hud_text('x')\n";
        let d = lint_lua_source(src);
        assert_eq!(d.len(), 1, "{:?}", d);
        assert_eq!(d[0].code, "LUA001_NO_SET_HUD_TEXT");
        assert_eq!(d[0].line, 3);
    }

    #[test]
    fn migrated_templates_have_no_errors() {
        for name in [
            "fps_controller",
            "third_person_controller",
            "top_down_controller",
            "vehicle_controller",
        ] {
            let path = format!("assets/scripts/templates/{}.lua", name);
            let diags = lint_lua_file(&path).expect("template should exist");
            let errors: Vec<_> = diags.iter().filter(|d| d.severity == "error").collect();
            assert!(errors.is_empty(), "{}: {:?}", path, errors);
        }
    }
}
