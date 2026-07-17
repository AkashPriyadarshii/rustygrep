mod common;

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn start_mcp() -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_rustygrep"))
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start rustygrep mcp")
}

fn send_request(child: &mut std::process::Child, request: &serde_json::Value) -> Option<serde_json::Value> {
    let stdin = child.stdin.as_mut()?;
    let stdout = child.stdout.as_mut()?;

    let msg = serde_json::to_string(request).unwrap();
    writeln!(stdin, "{}", msg).ok()?;
    stdin.flush().ok()?;

    let mut reader = BufReader::new(stdout);
    let mut response_line = String::new();
    reader.read_line(&mut response_line).ok()?;
    serde_json::from_str(&response_line).ok()
}

#[test]
fn mcp_initialize() {
    let mut child = start_mcp();
    let response = send_request(&mut child, &serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    child.kill().ok();

    let resp = response.unwrap();
    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 1);
    assert!(resp["result"]["capabilities"]["tools"].is_object());
    assert_eq!(resp["result"]["serverInfo"]["name"], "rustygrep");
}

#[test]
fn mcp_tools_list() {
    let mut child = start_mcp();

    send_request(&mut child, &serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    let response = send_request(&mut child, &serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    }));

    child.kill().ok();

    let resp = response.unwrap();
    let tools = resp["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 3);

    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"rustygrep_search"));
    assert!(names.contains(&"rustygrep_files"));
    assert!(names.contains(&"rustygrep_count"));
}

#[test]
fn mcp_search_tool() {
    let mut child = start_mcp();

    send_request(&mut child, &serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    let response = send_request(&mut child, &serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "rustygrep_search",
            "arguments": {
                "pattern": "fn",
                "path": ".",
                "format": "llm"
            }
        }
    }));

    child.kill().ok();

    let resp = response.unwrap();
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(!text.is_empty());
}

#[test]
fn mcp_files_tool() {
    let mut child = start_mcp();

    send_request(&mut child, &serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    let response = send_request(&mut child, &serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "rustygrep_files",
            "arguments": {
                "pattern": "fn",
                "path": "."
            }
        }
    }));

    child.kill().ok();

    let resp = response.unwrap();
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(!text.is_empty());
    assert!(!text.contains("Error"));
}

#[test]
fn mcp_count_tool() {
    let mut child = start_mcp();

    send_request(&mut child, &serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    let response = send_request(&mut child, &serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "rustygrep_count",
            "arguments": {
                "pattern": "fn",
                "path": "."
            }
        }
    }));

    child.kill().ok();

    let resp = response.unwrap();
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains(":"));
}

#[test]
fn mcp_unknown_method() {
    let mut child = start_mcp();

    send_request(&mut child, &serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    let response = send_request(&mut child, &serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "unknown/method"
    }));

    child.kill().ok();

    let resp = response.unwrap();
    assert!(resp["error"].is_object());
    assert_eq!(resp["error"]["code"], -32601);
}
