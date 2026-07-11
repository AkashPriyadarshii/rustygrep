use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

use crate::cli::Cli;
use crate::search::SearchEngine;
use crate::walker::FileWalker;

pub fn run() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();

    let mut buffer = String::new();

    loop {
        buffer.clear();
        match reader.read_line(&mut buffer) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }

        let line = buffer.trim();
        if line.is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let response = handle_request(&request);
        if let Some(resp) = response {
            let _ = writeln!(writer, "{}", serde_json::to_string(&resp).unwrap());
            let _ = writer.flush();
        }
    }
}

fn handle_request(request: &Value) -> Option<Value> {
    let method = request.get("method")?.as_str()?;
    let id = request.get("id");

    match method {
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "rustygrep",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        })),
        "notifications/initialized" => None,
        "tools/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [
                    {
                        "name": "rustygrep_search",
                        "description": "Search for a regex pattern in files. Returns matching lines with file paths and line numbers.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "pattern": {
                                    "type": "string",
                                    "description": "Regex pattern to search for"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "Directory or file to search in (default: current directory)",
                                    "default": "."
                                },
                                "format": {
                                    "type": "string",
                                    "enum": ["llm", "json", "pretty"],
                                    "description": "Output format (default: llm)",
                                    "default": "llm"
                                },
                                "type": {
                                    "type": "string",
                                    "description": "Filter by file type (e.g. rs, py, js)"
                                },
                                "max_results": {
                                    "type": "integer",
                                    "description": "Maximum number of files to return (0 = unlimited)",
                                    "default": 0
                                }
                            },
                            "required": ["pattern"]
                        }
                    },
                    {
                        "name": "rustygrep_files",
                        "description": "List files containing a pattern (no line content). Fast file discovery.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "pattern": {
                                    "type": "string",
                                    "description": "Regex pattern to search for"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "Directory or file to search in (default: current directory)",
                                    "default": "."
                                },
                                "type": {
                                    "type": "string",
                                    "description": "Filter by file type (e.g. rs, py, js)"
                                }
                            },
                            "required": ["pattern"]
                        }
                    },
                    {
                        "name": "rustygrep_count",
                        "description": "Count matches per file for a pattern.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "pattern": {
                                    "type": "string",
                                    "description": "Regex pattern to search for"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "Directory or file to search in (default: current directory)",
                                    "default": "."
                                },
                                "type": {
                                    "type": "string",
                                    "description": "Filter by file type (e.g. rs, py, js)"
                                }
                            },
                            "required": ["pattern"]
                        }
                    }
                ]
            }
        })),
        "tools/call" => {
            let params = request.get("params")?;
            let tool_name = params.get("name")?.as_str()?;
            let args = params.get("arguments").cloned().unwrap_or(json!({}));

            let result = call_tool(tool_name, &args);

            Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [
                        {
                            "type": "text",
                            "text": result
                        }
                    ]
                }
            }))
        }
        _ => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": format!("Method not found: {}", method)
            }
        })),
    }
}

fn call_tool(name: &str, args: &Value) -> String {
    let pattern = match args.get("pattern").and_then(|p| p.as_str()) {
        Some(p) => p.to_string(),
        None => return "Error: pattern is required".to_string(),
    };

    let path = args
        .get("path")
        .and_then(|p| p.as_str())
        .unwrap_or(".")
        .to_string();

    let file_type = args
        .get("type")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());

    let max_results = args
        .get("max_results")
        .and_then(|m| m.as_u64())
        .unwrap_or(0) as usize;

    let format = args.get("format").and_then(|f| f.as_str()).unwrap_or("llm");

    // Build a fake CLI to reuse existing search engine
    let cli = Cli {
        pattern: Some(pattern.clone()),
        paths: vec![std::path::PathBuf::from(&path)],
        format: crate::cli::OutputFormat::Llm,
        llm: false,
        json: false,
        json_file: false,
        llm_budget: None,
        llm_no_truncate: false,
        top: None,
        ignore_case: false,
        word_regexp: false,
        count: false,
        files_with_matches: name == "rustygrep_files",
        after_context: None,
        before_context: None,
        context: None,
        file_type: file_type.clone(),
        file_type_not: None,
        max_columns: 500,
        hidden: false,
        no_ignore: false,
        no_binary: false,
        invert_match: false,
        threads: 0,
        no_color: true,
        max_matches: max_results,
        subcommand: None,
    };

    let engine = match SearchEngine::new(&cli) {
        Ok(e) => e,
        Err(err) => return format!("Error: invalid pattern: {}", err),
    };

    let files = FileWalker::new(cli.paths.clone(), false, false, false, file_type, None, 0).walk();

    if files.is_empty() {
        return "No files found matching criteria.".to_string();
    }

    let results = engine.search(&files);

    if results.is_empty() {
        return "No matches found.".to_string();
    }

    let count_only = name == "rustygrep_count";

    if name == "rustygrep_files" {
        let files: Vec<&str> = results.iter().map(|r| r.path.as_str()).collect();
        return files.join("\n");
    }

    if count_only {
        let lines: Vec<String> = results
            .iter()
            .map(|r| format!("{}:{}", r.path, r.matches.len()))
            .collect();
        return lines.join("\n");
    }

    // For search: use the requested format
    // Format output based on requested format
    match format {
        "json" => {
            let lines: Vec<String> = results
                .iter()
                .flat_map(|r| {
                    r.matches.iter().map(move |m| {
                        serde_json::to_string(&serde_json::json!({
                            "path": m.path,
                            "line": m.line_number,
                            "match_text": m.line,
                        }))
                        .unwrap()
                    })
                })
                .collect();
            lines.join("\n")
        }
        "pretty" => {
            let lines: Vec<String> = results
                .iter()
                .flat_map(|r| {
                    r.matches
                        .iter()
                        .map(move |m| format!("{}:{}:{}", m.path, m.line_number, m.line))
                })
                .collect();
            lines.join("\n")
        }
        _ => {
            // LLM format
            let mut output = String::new();
            let mut current_file = String::new();
            for file_match in &results {
                for m in &file_match.matches {
                    if m.path != current_file {
                        if !current_file.is_empty() {
                            output.push('\n');
                        }
                        current_file = m.path.clone();
                        output.push_str(&format!("--- {}\n", current_file));
                    }
                    let content = if m.line.len() > 120 {
                        let mut end = 120;
                        while end > 0 && !m.line.is_char_boundary(end) {
                            end -= 1;
                        }
                        format!("{}...", &m.line[..end])
                    } else {
                        m.line.clone()
                    };
                    output.push_str(&format!("{}:{}\n", m.line_number, content));
                }
            }
            let total: usize = results.iter().map(|r| r.matches.len()).sum();
            output.push_str(&format!(
                "\n--- {} match{} in {} file{}\n",
                total,
                if total == 1 { "" } else { "es" },
                results.len(),
                if results.len() == 1 { "" } else { "s" }
            ));
            output
        }
    }
}
