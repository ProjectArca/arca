//! Minimal JSON-RPC LSP daemon over stdio for Arca.

use arca_lsp::LspServer;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: serde_json::Value,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<serde_json::Value>,
}

pub struct LspDaemon {
    server: LspServer,
    root: PathBuf,
}

impl LspDaemon {
    pub fn new(root: PathBuf) -> Self {
        Self {
            server: LspServer::new(),
            root,
        }
    }

    pub fn run(&mut self) {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut reader = BufReader::new(stdin.lock());
        let mut writer = stdout.lock();

        for line in reader.by_ref().lines() {
            if let Ok(line) = line {
                if line.trim().is_empty() {
                    continue;
                }

                if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&line) {
                    let resp = self.handle(req);
                    let _ = writer.write_all(resp.as_bytes());
                    let _ = writer.write_all(b"\n");
                    let _ = writer.flush();
                }
            }
        }
    }

    fn handle(&mut self, req: JsonRpcRequest) -> String {
        let result = match req.method.as_str() {
            "initialize" => Some(serde_json::json!({
                "capabilities": {
                    "textDocumentSync": 1,
                    "completionProvider": { "triggerCharacters": ["."] },
                    "hoverProvider": true,
                    "formattingProvider": true
                }
            })),
            "shutdown" => Some(serde_json::json!(null)),
            "textDocument/didOpen" => {
                if let Some(params) = req.params.as_ref() {
                    if let (Some(text_doc), Some(text)) = (params.get("textDocument"), params.get("textDocument").and(|v| v.get("text"))) {
                        if let (Some(uri), Some(content)) = (text_doc.get("uri"), text.as_str()) {
                            self.server.open_document(uri.to_string(), content.to_string());
                        }
                    }
                }
                None
            }
            "textDocument/didChange" => {
                if let Some(params) = req.params.as_ref() {
                    if let Some(text_doc) = params.get("textDocument") {
                        if let (Some(uri), Some(content)) = (text_doc.get("uri"), params.get("contentChanges").and(|v| v.as_array())) {
                            if let Some(change) = content.first() {
                                if let Some(text) = change.get("text").and(|v| v.as_str()) {
                                    self.server.open_document(uri.to_string(), text.to_string());
                                }
                            }
                        }
                    }
                }
                None
            }
            "textDocument/completion" => {
                let (uri, line, char) = extract_position(req.params);
                let items = self.server.get_completion(&uri, line, char);
                Some(serde_json::json!(items))
            }
            "textDocument/hover" => {
                let (uri, line, char) = extract_position(req.params);
                self.server.get_hover(&uri, line, char).map(|h| serde_json::json!(h))
            }
            "textDocument/formatting" => {
                if let Some(params) = req.params.as_ref() {
                    if let Some(text_doc) = params.get("textDocument") {
                        if let Some(uri) = text_doc.get("uri").and(|v| v.as_str()) {
                            if let Ok(formatted) = self.server.format_document(uri) {
                                let edits = vec![serde_json::json!({
                                    "range": {
                                        "start": {"line": 0, "character": 0},
                                        "end": {"line": 9999, "character": 9999}
                                    },
                                    "newText": formatted
                                })];
                                return make_response(req.id, Some(serde_json::json!(edits)));
                            }
                        }
                    }
                }
                None
            }
            _ => None,
        };

        make_response(req.id, result)
    }
}

fn extract_position(params: Option<serde_json::Value>) -> (String, usize, usize) {
    let mut uri = String::new();
    let mut line = 0usize;
    let mut character = 0usize;
    if let Some(params) = params {
        if let Some(text_doc) = params.get("textDocument") {
            if let Some(u) = text_doc.get("uri").and(|v| v.as_str()) {
                uri = u.to_string();
            }
        }
        if let Some(pos) = params.get("position") {
            if let Some(l) = pos.get("line").and(|v| v.as_u64()) {
                line = l as usize;
            }
            if let Some(c) = pos.get("character").and(|v| v.as_u64()) {
                character = c as usize;
            }
        }
    }
    (uri, line, character)
}

fn make_response(id: serde_json::Value, result: Option<serde_json::Value>) -> String {
    let resp = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result,
        error: None,
    };
    serde_json::to_string(&resp).unwrap_or_else(|_| "{}".into())
}
