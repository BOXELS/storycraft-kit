//! Where the Craft suite and STORYCRAFT keep things on this computer.
//!
//! macOS: `~/Library/Application Support/ArtCraft/` holds `flags.toml` and `mcp/` (the launcher's
//! `apps.json` and the gateway copy in `mcp/bin`). Each env override mirrors the suite's own.

use std::path::PathBuf;

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from)
}

pub fn home() -> Option<PathBuf> {
    env_path("HOME").or_else(dirs::home_dir)
}

/// `ArtCraft` in the per-user local configuration folder, as `craft-flags` and `craft-ai-agents` use.
pub fn artcraft_dir() -> Option<PathBuf> {
    dirs::config_local_dir().map(|d| d.join("ArtCraft"))
}

/// `CRAFT_FLAGS_FILE`, else `ArtCraft/flags.toml`.
pub fn flags_file() -> Option<PathBuf> {
    env_path("CRAFT_FLAGS_FILE").or_else(|| artcraft_dir().map(|d| d.join("flags.toml")))
}

/// `CRAFT_MCP_DIR`, else `ArtCraft/mcp`.
pub fn mcp_dir() -> Option<PathBuf> {
    craft_ai_agents::agents_dir()
}

/// The gateway copy the launcher writes for MCP clients to start.
pub fn gateway_copy() -> Option<PathBuf> {
    let name = if cfg!(windows) { "craft-ai-mcp.exe" } else { "craft-ai-mcp" };
    mcp_dir().map(|d| d.join("bin").join(name))
}

/// `SC_JOBS_DIR`, else `~/StoryCraft/jobs`.
pub fn jobs_dir() -> Option<PathBuf> {
    env_path("SC_JOBS_DIR").or_else(|| home().map(|h| h.join("StoryCraft").join("jobs")))
}

/// `SC_CACHE_DIR`, else the platform cache folder (`~/Library/Caches/storycraft` on macOS).
pub fn cache_dir() -> Option<PathBuf> {
    env_path("SC_CACHE_DIR").or_else(|| dirs::cache_dir().map(|d| d.join("storycraft")))
}

/// `SC_RECIPES_DIR`, else `./recipes` when it exists, else `~/.config/storycraft/recipes`.
pub fn recipes_dir() -> Option<PathBuf> {
    if let Some(p) = env_path("SC_RECIPES_DIR") {
        return Some(p);
    }
    let local = PathBuf::from("recipes");
    if local.is_dir() {
        return Some(local);
    }
    home().map(|h| h.join(".config").join("storycraft").join("recipes"))
}

/// The app log folder the bug collector reads: `~/Library/Application Support/<AppName>/logs`.
pub fn app_logs_dir(app_name: &str) -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join(app_name).join("logs"))
}
