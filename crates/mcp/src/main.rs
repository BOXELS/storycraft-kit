//! `storycraft-mcp`: stdio MCP server. Logs go to stderr; stdout carries only protocol messages.

use std::io::{BufRead, Write};

fn main() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if let Some(reply) = storycraft_mcp::handle_line(&line)
            && writeln!(stdout, "{reply}").and_then(|()| stdout.flush()).is_err()
        {
            break;
        }
    }
}
