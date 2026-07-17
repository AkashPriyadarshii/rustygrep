use std::fs;
use std::path::{Path, PathBuf};

pub fn run() {
    println!("rustygrep init\n");

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Claude Code
    setup_claude_code(&cwd);

    // Cursor
    setup_cursor(&cwd);

    println!("\nDone. Restart your agent to pick up changes.");
}

fn setup_claude_code(cwd: &Path) {
    let claude_dir = cwd.join(".claude");
    if !claude_dir.exists() {
        println!("  [skip] Claude Code — no .claude/ directory (run `claude init` first)");
        return;
    }

    let settings_path = claude_dir.join("settings.json");
    let mcp_path = claude_dir.join("mcp.json");

    // Write MCP server config
    let mcp_config = serde_json::json!({
        "mcpServers": {
            "rustygrep": {
                "command": "rustygrep",
                "args": ["mcp"]
            }
        }
    });

    if mcp_path.exists() {
        // Merge with existing
        let existing: serde_json::Value = fs::read_to_string(&mcp_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(serde_json::json!({"mcpServers": {}}));

        if existing
            .get("mcpServers")
            .and_then(|s| s.get("rustygrep"))
            .is_some()
        {
            println!("  [ok] Claude Code — rustygrep MCP already configured");
            return;
        }

        let mut merged = existing;
        merged["mcpServers"]["rustygrep"] = serde_json::json!({
            "command": "rustygrep",
            "args": ["mcp"]
        });

        fs::write(&mcp_path, serde_json::to_string_pretty(&merged).unwrap()).ok();
    } else {
        fs::write(
            &mcp_path,
            serde_json::to_string_pretty(&mcp_config).unwrap(),
        )
        .ok();
    }

    println!("  [ok] Claude Code — added rustygrep MCP to .claude/mcp.json");

    // Also suggest adding to settings.json if it exists
    if settings_path.exists() {
        println!("  [tip] Add to .claude/settings.json for grep replacement:");
        println!("        \"preferred_grep\": \"rustygrep\"");
    }
}

fn setup_cursor(cwd: &Path) {
    let vscode_dir = cwd.join(".vscode");
    if !vscode_dir.exists() {
        return;
    }

    let settings_path = vscode_dir.join("settings.json");

    let _rustygrep_setting = serde_json::json!({
        "search.searchEditor.singleClick": "open",
        "search.mode": "reuseEditor"
    });

    if settings_path.exists() {
        let existing: serde_json::Value = fs::read_to_string(&settings_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(serde_json::json!({}));

        if existing.get("search.searchEditor.singleClick").is_some() {
            println!("  [ok] Cursor — search settings already configured");
            return;
        }
    }

    println!("  [tip] For Cursor integration, add to .vscode/settings.json:");
    println!("        \"search.searchEditor.singleClick\": \"open\"");
    println!("        \"search.mode\": \"reuseEditor\"");
}
