//! The Craft suite's feature flags (`ArtCraft/flags.toml`, `[flags] AI-MCP = true`).
//!
//! Every Craft app and the launcher read this file. Writing keeps every other key and comment.
//! `CRAFT_FLAGS` (`AI-MCP,-AI-MODELS`) overrides the file for one run and wins over it.

use std::path::Path;

use toml_edit::{DocumentMut, Item, Table, value};

use crate::{Error, Result};

/// The flag that lets AI agents use the Craft apps through the launcher's MCP switch.
pub const AI_MCP: &str = "AI-MCP";

/// Flag ids are upper case letters, digits and `-`, as `craft-flags` checks them.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FlagState {
    pub id: String,
    /// What the file says (`None`: not set there).
    pub file: Option<bool>,
    /// What `CRAFT_FLAGS` says for this run (`None`: not named).
    pub env: Option<bool>,
    pub enabled: bool,
}

fn parse_bool(v: &str) -> Option<bool> {
    match v.trim().to_ascii_lowercase().as_str() {
        "" | "1" | "true" | "on" | "yes" => Some(true),
        "0" | "false" | "off" | "no" => Some(false),
        _ => None,
    }
}

/// What a `CRAFT_FLAGS` value says about `id`.
pub fn env_value(env: &str, id: &str) -> Option<bool> {
    let mut found = None;
    for part in env.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (name, on) = match part.split_once('=') {
            Some((n, v)) => (n.trim(), parse_bool(v)),
            None => match part.strip_prefix('-') {
                Some(n) => (n, Some(false)),
                None => (part.strip_prefix('+').unwrap_or(part), Some(true)),
            },
        };
        if name == id {
            found = on;
        }
    }
    found
}

fn read_doc(path: &Path) -> Result<DocumentMut> {
    match std::fs::read_to_string(path) {
        Ok(text) => text.parse::<DocumentMut>().map_err(|e| Error::Other(format!("{} does not parse: {e}", path.display()))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(DocumentMut::new()),
        Err(e) => Err(Error::io(path, e)),
    }
}

pub fn file_value(path: &Path, id: &str) -> Result<Option<bool>> {
    let doc = read_doc(path)?;
    Ok(doc.get("flags").and_then(|t| t.get(id)).and_then(|v| v.as_bool()))
}

pub fn state(path: &Path, env: Option<&str>, id: &str) -> Result<FlagState> {
    let file = file_value(path, id)?;
    let env = env.and_then(|e| env_value(e, id));
    Ok(FlagState { id: id.to_string(), file, env, enabled: env.or(file).unwrap_or(false) })
}

/// Set `id` in the file (creating the file and its folder), keeping everything else.
pub fn set(path: &Path, id: &str, on: bool) -> Result<()> {
    if !valid_id(id) {
        return Err(Error::Other(format!("`{id}` is not a flag id (upper case letters, digits and -)")));
    }
    let mut doc = read_doc(path)?;
    if !doc.contains_table("flags") {
        doc.insert("flags", Item::Table(Table::new()));
    }
    let Some(flags) = doc.get_mut("flags").and_then(Item::as_table_mut) else {
        return Err(Error::Other(format!("{}: `flags` is not a table", path.display())));
    };
    flags.insert(id, value(on));
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
    }
    std::fs::write(path, doc.to_string()).map_err(|e| Error::io(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_overrides_follow_craft_flags_syntax() {
        assert_eq!(env_value("AI-MCP", AI_MCP), Some(true));
        assert_eq!(env_value("AI-MODELS,-AI-MCP", AI_MCP), Some(false));
        assert_eq!(env_value("AI-MCP=0", AI_MCP), Some(false));
        assert_eq!(env_value("AI-MODELS", AI_MCP), None);
    }

    #[test]
    fn setting_a_flag_keeps_the_rest_of_the_file() -> Result<()> {
        let dir = tempfile::tempdir().map_err(|e| Error::Other(e.to_string()))?;
        let path = dir.path().join("ArtCraft").join("flags.toml");
        std::fs::create_dir_all(dir.path().join("ArtCraft")).map_err(|e| Error::io(dir.path(), e))?;
        std::fs::write(&path, "# mine\n[flags]\nAI-MODELS = true\n").map_err(|e| Error::io(&path, e))?;
        set(&path, AI_MCP, true)?;
        let text = std::fs::read_to_string(&path).map_err(|e| Error::io(&path, e))?;
        assert!(text.contains("# mine") && text.contains("AI-MODELS = true") && text.contains("AI-MCP = true"), "{text}");
        assert!(!state(&path, Some("-AI-MCP"), AI_MCP)?.enabled);
        assert!(state(&path, None, AI_MCP)?.enabled);
        Ok(())
    }

    #[test]
    fn a_missing_file_is_created() -> Result<()> {
        let dir = tempfile::tempdir().map_err(|e| Error::Other(e.to_string()))?;
        let path = dir.path().join("new").join("flags.toml");
        assert_eq!(file_value(&path, AI_MCP)?, None);
        set(&path, AI_MCP, true)?;
        assert_eq!(file_value(&path, AI_MCP)?, Some(true));
        Ok(())
    }
}
