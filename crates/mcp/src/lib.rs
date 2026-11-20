//! `storycraft-mcp`: STORYCRAFT's workflow tools over MCP (JSON-RPC 2.0, one message per line, stdio).
//!
//! App-level tools (open, edit, export inside one app) come from the ArtCraft gateway
//! (`craft-ai-mcp`); this server adds the workflow layer above it: setup checks, recipes, bug drafts.
//! Tools that write files only write inside a job folder.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::{Value, json};
use storycraft_core::{bug, doctor, engine, paths, recipe};

pub const PROTOCOL_VERSION: &str = "2025-06-18";

fn tools() -> Value {
    json!([
        {
            "name": "sc_doctor",
            "description": "Check this Mac is ready for agents to drive the ArtCraft apps (AI-MCP flag, launcher agents switch, PhotoCraft/VectorCraft/FilmCraft CLIs, MCP clients). Reads only; returns each check with a hint.",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "sc_list_recipes",
            "description": "List STORYCRAFT recipes (automated workflows such as youtube-thumbnail and icon-set) with their inputs.",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "sc_run_recipe",
            "description": "Run a recipe by id. Inputs are strings (file inputs are absolute paths). Each run writes only into its own new job folder and returns the job report: steps, app versions, output checks. Use dry_run to see the exact app commands first.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "recipe": { "type": "string" },
                    "inputs": { "type": "object", "additionalProperties": { "type": "string" } },
                    "dry_run": { "type": "boolean" }
                },
                "required": ["recipe"],
                "additionalProperties": false
            },
            "annotations": { "readOnlyHint": false, "destructiveHint": false }
        },
        {
            "name": "sc_bug_draft",
            "description": "Write a scrubbed bug report draft (bug-draft.md) from a failed job folder. Never files anything: show the draft to the user and file only after they approve that specific issue.",
            "inputSchema": {
                "type": "object",
                "properties": { "job_dir": { "type": "string" } },
                "required": ["job_dir"],
                "additionalProperties": false
            },
            "annotations": { "readOnlyHint": false, "destructiveHint": false }
        }
    ])
}

fn text_result(value: &Value, is_error: bool) -> Value {
    let text = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    json!({ "content": [{ "type": "text", "text": text }], "structuredContent": value, "isError": is_error })
}

fn err_result(message: impl std::fmt::Display) -> Value {
    json!({ "content": [{ "type": "text", "text": message.to_string() }], "isError": true })
}

fn call(name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "sc_doctor" => {
            let report = doctor::run(&doctor::Options::default());
            Ok(text_result(&serde_json::to_value(&report).map_err(|e| e.to_string())?, false))
        }
        "sc_list_recipes" => {
            let dir = paths::recipes_dir().ok_or("no recipes folder; set SC_RECIPES_DIR")?;
            let list: Vec<Value> = recipe::list(&dir)
                .map_err(|e| e.to_string())?
                .into_iter()
                .map(|r| match r {
                    Ok(r) => json!({
                        "id": r.id,
                        "summary": r.summary,
                        "inputs": r.inputs.iter().map(|(k, i)| (k.clone(), json!({ "type": i.kind, "required": i.required, "default": i.default, "help": i.help }))).collect::<serde_json::Map<_, _>>(),
                        "apps": r.requires.keys().collect::<Vec<_>>(),
                    }),
                    Err(e) => json!({ "error": e.to_string() }),
                })
                .collect();
            Ok(text_result(&json!({ "recipes": list }), false))
        }
        "sc_run_recipe" => {
            let id = args.get("recipe").and_then(Value::as_str).ok_or("`recipe` is required")?;
            let inputs: BTreeMap<String, String> = match args.get("inputs") {
                Some(Value::Object(m)) => m
                    .iter()
                    .map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())).ok_or(format!("input `{k}` must be a string")))
                    .collect::<Result<_, _>>()?,
                None | Some(Value::Null) => BTreeMap::new(),
                Some(_) => return Err("`inputs` must be an object of strings".into()),
            };
            let dry_run = args.get("dry_run").and_then(Value::as_bool).unwrap_or(false);
            let r = recipe::Recipe::load(&recipe::resolve(id).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            match engine::run(&r, &engine::RunOptions { inputs, job_dir: None, dry_run, app_overrides: BTreeMap::new() }) {
                Ok(report) => Ok(text_result(&serde_json::to_value(&report).map_err(|e| e.to_string())?, false)),
                Err(e) => Ok(err_result(format!(
                    "{e}\n(a failed app step leaves failure.json in the job folder; sc_bug_draft can draft a report)"
                ))),
            }
        }
        "sc_bug_draft" => {
            let dir = args.get("job_dir").and_then(Value::as_str).ok_or("`job_dir` is required")?;
            match bug::draft(&PathBuf::from(dir)) {
                Ok(d) => Ok(text_result(&serde_json::to_value(&d).map_err(|e| e.to_string())?, false)),
                Err(e) => Ok(err_result(e)),
            }
        }
        other => Err(format!("unknown tool `{other}`")),
    }
}

fn error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message.into() } })
}

/// Answer one JSON-RPC message; `None` for notifications.
pub fn handle(msg: &Value) -> Option<Value> {
    let method = msg.get("method").and_then(Value::as_str);
    let id = msg.get("id").cloned()?;
    let Some(method) = method else {
        return Some(error(id, -32600, "not a request"));
    };
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    let result = match method {
        "initialize" => {
            let version = params.get("protocolVersion").and_then(Value::as_str).unwrap_or(PROTOCOL_VERSION);
            json!({
                "protocolVersion": version,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "storycraft-mcp", "version": env!("CARGO_PKG_VERSION") },
                "instructions": "STORYCRAFT workflow tools for the ArtCraft apps. Run sc_doctor first. For single-app edits use the ArtCraft gateway's craft_* tools."
            })
        }
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tools() }),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            match call(name, &args) {
                Ok(v) => v,
                Err(e) if e.starts_with("unknown tool") => return Some(error(id, -32602, e)),
                Err(e) => err_result(e),
            }
        }
        other => return Some(error(id, -32601, format!("method `{other}` not found"))),
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

/// Answer one line of input; `None` when nothing should be written back.
pub fn handle_line(line: &str) -> Option<String> {
    if line.trim().is_empty() {
        return None;
    }
    let reply = match serde_json::from_str::<Value>(line) {
        Ok(msg) => handle(&msg)?,
        Err(e) => error(Value::Null, -32700, format!("parse error: {e}")),
    };
    Some(reply.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(line: &str) -> Value {
        handle_line(line).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null)
    }

    #[test]
    fn initialize_and_list_tools() {
        let init = ask(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26"}}"#);
        assert_eq!(init["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(init["result"]["serverInfo"]["name"], "storycraft-mcp");
        assert!(handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none());
        let list = ask(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
        let names: Vec<&str> =
            list["result"]["tools"].as_array().map(|t| t.iter().filter_map(|x| x["name"].as_str()).collect()).unwrap_or_default();
        assert_eq!(names, ["sc_doctor", "sc_list_recipes", "sc_run_recipe", "sc_bug_draft"]);
    }

    #[test]
    fn bad_input_gets_json_rpc_errors() {
        assert_eq!(ask("{nope")["error"]["code"], -32700);
        assert_eq!(ask(r#"{"jsonrpc":"2.0","id":3,"method":"nope"}"#)["error"]["code"], -32601);
        assert_eq!(ask(r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"nope"}}"#)["error"]["code"], -32602);
        let missing = ask(r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"sc_run_recipe","arguments":{}}}"#);
        assert_eq!(missing["result"]["isError"], true);
    }
}
