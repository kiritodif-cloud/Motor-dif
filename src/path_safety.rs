//! Workspace-relative path validation for untrusted MCP input.
//!
//! Every filesystem path accepted from MCP is validated here before it reaches
//! an engine handler or a direct `std::fs` call. Absolute paths, Windows drive
//! prefixes, UNC paths and `..` components are rejected instead of normalized.

use std::path::{Component, Path, PathBuf};

/// Validates a relative path such as `assets/scene.json`.
///
/// `extensions` may contain lowercase extensions without the leading dot; pass
/// an empty slice to allow any extension.
pub fn validate_relative(path: &str, extensions: &[&str]) -> Result<String, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Path cannot be empty".to_string());
    }
    if trimmed.contains('\0') {
        return Err("Path cannot contain NUL bytes".to_string());
    }

    let normalized = trimmed.replace('\\', "/");
    if normalized.starts_with('/') || normalized.starts_with("//") {
        return Err(format!("Absolute paths are not allowed: '{}'", path));
    }

    let relative = normalized
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>();
    if relative.is_empty() {
        return Err("Path cannot be empty".to_string());
    }
    if relative.iter().any(|part| *part == "..") {
        return Err(format!(
            "Parent directory components are not allowed: '{}'",
            path
        ));
    }
    let relative_str = relative.join("/");

    let path_obj = Path::new(&relative_str);
    for component in path_obj.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!("Unsafe path component in '{}'", path));
            }
        }
    }

    if !extensions.is_empty() {
        let ext = path_obj
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !extensions.iter().any(|e| ext == *e) {
            return Err(format!(
                "Path '{}' must end with one of: {}",
                path,
                extensions.join(", ")
            ));
        }
    }

    ensure_inside_workspace(&relative_str)?;
    Ok(relative_str)
}

/// Validates a single filename or ID used to build a path server-side.
pub fn validate_name(name: &str, extensions: &[&str]) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Name cannot be empty".to_string());
    }
    if trimmed.contains('\0')
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains(':')
    {
        return Err(format!(
            "Name cannot contain path separators or drive prefixes: '{}'",
            name
        ));
    }
    if trimmed.split('/').any(|p| p == "..") || trimmed == ".." || trimmed == "." {
        return Err(format!(
            "Parent directory components are not allowed: '{}'",
            name
        ));
    }

    if !extensions.is_empty() {
        let ext = Path::new(trimmed)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !extensions.iter().any(|e| ext == *e) {
            return Err(format!(
                "Name '{}' must end with one of: {}",
                name,
                extensions.join(", ")
            ));
        }
    }
    Ok(trimmed.to_string())
}

/// Resolves an already validated relative path to an absolute workspace path.
pub fn resolve_relative(path: &str) -> Result<PathBuf, String> {
    let relative = validate_relative(path, &[])?;
    Ok(crate::resolve_workspace_root().join(relative))
}

fn ensure_inside_workspace(relative: &str) -> Result<(), String> {
    let root = crate::resolve_workspace_root();
    let root_canonical = std::fs::canonicalize(&root)
        .map_err(|e| format!("Cannot resolve workspace root '{}': {}", root.display(), e))?;
    let candidate = root.join(relative);

    let mut current = candidate.as_path();
    loop {
        if let Ok(meta) = std::fs::symlink_metadata(current) {
            if meta.file_type().is_symlink() {
                let target = std::fs::canonicalize(current).map_err(|e| {
                    format!("Cannot resolve symlink '{}': {}", current.display(), e)
                })?;
                if !target.starts_with(&root_canonical) {
                    return Err(format!(
                        "Path '{}' escapes the workspace via a symlink",
                        relative
                    ));
                }
            }
        }
        match current.parent() {
            Some(parent) if parent >= root.as_path() => current = parent,
            _ => break,
        }
    }

    if let Ok(canonical) = std::fs::canonicalize(&candidate) {
        if !canonical.starts_with(&root_canonical) {
            return Err(format!("Path '{}' escapes the workspace", relative));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_relative_paths() {
        assert_eq!(
            validate_relative("assets\\scene.json", &["json"]).unwrap(),
            "assets/scene.json"
        );
        assert_eq!(
            validate_relative("./assets/scripts/player.lua", &["lua"]).unwrap(),
            "assets/scripts/player.lua"
        );
        assert_eq!(
            validate_relative("scene.json", &["json"]).unwrap(),
            "scene.json"
        );
    }

    #[test]
    fn rejects_traversal_absolute_and_unc_paths() {
        for bad in [
            "../evil.json",
            "assets/../../evil.json",
            "assets\\..\\evil.json",
            "/tmp/evil.json",
            "C:\\evil.json",
            "\\\\server\\share\\evil.json",
            "assets/../scene.json",
        ] {
            assert!(
                validate_relative(bad, &["json"]).is_err(),
                "accepted '{}'",
                bad
            );
        }
    }

    #[test]
    fn validates_names_without_separators() {
        assert_eq!(validate_name("player", &[]).unwrap(), "player");
        assert_eq!(validate_name("theme.wav", &["wav"]).unwrap(), "theme.wav");
        for bad in ["../player", "..\\player", "C:player", "a/b", "player\0x"] {
            assert!(validate_name(bad, &[]).is_err(), "accepted '{}'", bad);
        }
    }

    #[test]
    fn rejects_disallowed_extensions_and_nul() {
        assert!(validate_relative("assets/scene.json", &["lua"]).is_err());
        assert!(validate_relative("assets/scene\0.json", &[]).is_err());
        assert!(validate_relative("", &[]).is_err());
    }

    #[test]
    fn resolves_inside_workspace() {
        let resolved = resolve_relative("assets/scene.json").unwrap();
        let root = crate::resolve_workspace_root();
        assert!(resolved.starts_with(root));
    }
}
