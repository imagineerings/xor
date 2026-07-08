use anyhow::{Context as _, Result, anyhow};
use mcp_tutorial::{TutorialCatalog, TutorialServer};
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};

fn main() -> Result<()> {
    let tutorials_dir = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("SIM_TUTORIAL_MCP_DIR").ok())
        .ok_or_else(|| anyhow!("usage: mcp-tutorial <tutorial-markdown-directory>"))?;
    let catalog = TutorialCatalog::load_from_dir(tutorials_dir)?;
    let server = TutorialServer::new(catalog);

    for line in io::stdin().lock().lines() {
        let line = line.context("reading JSON-RPC request")?;
        if line.trim().is_empty() {
            continue;
        }

        let response = handle_json_rpc_request(&server, &line);
        writeln!(io::stdout(), "{response}")?;
        io::stdout().flush()?;
    }

    Ok(())
}

fn handle_json_rpc_request(server: &TutorialServer, line: &str) -> Value {
    let request: Value = match serde_json::from_str(line) {
        Ok(request) => request,
        Err(error) => return error_response(Value::Null, -32700, error.to_string()),
    };
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request.get("method").and_then(Value::as_str);

    let result = match method {
        Some("initialize") => Ok(json!({
            "protocolVersion": "2025-11-25",
            "capabilities": server.capabilities(),
            "serverInfo": {
                "name": "sim-tutorial",
                "version": env!("CARGO_PKG_VERSION")
            }
        })),
        Some("tools/list") => Ok(json!({ "tools": server.tools() })),
        Some("tools/call") => {
            let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
            let name = params.get("name").and_then(Value::as_str);
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            match name {
                Some(name) => server.handle_tool_call(name, arguments).map(|result| {
                    json!({
                        "content": [
                            {
                                "type": "text",
                                "text": result.to_string()
                            }
                        ],
                        "isError": false
                    })
                }),
                None => Err(anyhow!("tools/call params must include a string name")),
            }
        }
        Some("ping") => Ok(json!({})),
        Some(method) => Err(anyhow!("unknown JSON-RPC method `{method}`")),
        None => Err(anyhow!("JSON-RPC request must include a string method")),
    };

    match result {
        Ok(result) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        }),
        Err(error) => error_response(id, -32603, error.to_string()),
    }
}

fn error_response(id: Value, code: i64, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}
