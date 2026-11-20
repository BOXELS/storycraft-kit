//! MCP client settings (`~/.cursor/mcp.json` and friends): which servers each client starts.
//!
//! The launcher adds `ArtCraft` (the `craft-ai-mcp --stdio` gateway). STORYCRAFT adds `STORYCRAFT`
//! (`storycraft-mcp`) beside it, never replacing or reordering anything else in the file.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use crate::{Error, Result, paths};

pub const ARTCRAFT_KEY: &str = "ArtCraft";
pub const STORYCRAFT_KEY: &str = "STORYCRAFT";

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Client {
    Cursor,
    ClaudeCode,
}

impl Client {
    pub const ALL: [Client; 2] = [Client::Cursor, Client::ClaudeCode];

    pub fn name(self) -> &'static str {
        match self {
            Client::Cursor => "Cursor",
            Client::ClaudeCode => "Claude Code",
        }
    }

    pub fn config_path(self) -> Option<PathBuf> {
        let home = paths::home()?;
        Some(match self {
            Client::Cursor => home.join(".cursor").join("mcp.json"),
            Client::ClaudeCode => home.join(".claude.json"),
        })
    }
}

fn read(path: &Path) -> Result<Value> {
    match std::fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => Ok(json!({})),
        Ok(text) => serde_json::from_str(&text).map_err(|e| Error::Other(format!("{} is not JSON: {e}", path.display()))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(Error::io(path, e)),
    }
}

/// The server entry named `key`, if the file has one.
pub fn server(path: &Path, key: &str) -> Result<Option<Value>> {
    Ok(read(path)?.get("mcpServers").and_then(|s| s.get(key)).cloned())
}

/// Add or replace the `key` server, keeping every other key in the file.
pub fn put_server(path: &Path, key: &str, command: &Path, args: &[&str]) -> Result<()> {
    let mut doc = read(path)?;
    let Some(root) = doc.as_object_mut() else {
        return Err(Error::Other(format!("{} is not a JSON object", path.display())));
    };
    let servers = root.entry("mcpServers").or_insert_with(|| Value::Object(Map::new()));
    let Some(servers) = servers.as_object_mut() else {
        return Err(Error::Other(format!("{}: mcpServers is not an object", path.display())));
    };
    servers.insert(key.to_string(), json!({ "command": command, "args": args }));
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
    }
    let text = serde_json::to_string_pretty(&doc)?;
    std::fs::write(path, text + "\n").map_err(|e| Error::io(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adding_storycraft_keeps_artcraft() -> Result<()> {
        let dir = tempfile::tempdir().map_err(|e| Error::Other(e.to_string()))?;
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, r#"{"mcpServers":{"ArtCraft":{"command":"/x/craft-ai-mcp","args":["--stdio"]}},"other":1}"#)
            .map_err(|e| Error::io(&path, e))?;
        put_server(&path, STORYCRAFT_KEY, Path::new("/usr/local/bin/storycraft-mcp"), &[])?;
        assert!(server(&path, ARTCRAFT_KEY)?.is_some());
        assert!(server(&path, STORYCRAFT_KEY)?.is_some());
        assert_eq!(read(&path)?["other"], 1);
        Ok(())
    }
}
