//! Draco MCP client 骨架 — 對「以 System User 運行的可觀測性 Draco MCP
//! Server」（Prometheus / OTEL / Jaeger / pprof scraper）發起 MCP-over-HTTP
//! 呼叫。
//!
//! Slice 0 只落 framing 骨架（initialize handshake + tools/call），不做真呼叫
//! —— ingest 主路徑是 file-based import（零 Draco 介面風險）。Slice 1/2
//! 接 Draco 4 tools 時在此擴充。
//!
//! Draco 的 MCP-over-HTTP handshake 與 CRG 同構（`initialize` → session id →
//! `tools/call`），framing 沿用 code-review-graph client 的實測格式。

use serde_json::{json, Value};
use std::time::Duration;

/// Draco 可觀測性 MCP tools — Slice 0 僅確認 §5.2 提及的
/// `fetch_top_hotspots`；完整 4 tools 清單於 Slice 1 probe 時補齊。
pub const DRACO_TOOLS: [&str; 1] = ["fetch_top_hotspots"];

/// 預設 Draco base url（`DRACO_BASE_URL` 環境變數覆寫）。
///
/// 注意：port 未經 probe 確認，Slice 1 實測 Draco MCP 後修正（CRG 的
/// 9877 不保證適用）。
#[must_use]
pub fn default_draco_url() -> String {
    std::env::var("DRACO_BASE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:9876/mcp".to_string())
}

/// Draco MCP client（streamable HTTP transport）。
#[derive(Debug, Clone)]
pub struct DracoMcpClient {
    base_url: String,
    /// initialize 拿到的 session id（`Mcp-Session-Id` header）。
    session_id: Option<String>,
}

impl DracoMcpClient {
    /// 建立 client（`base_url` 如 `http://127.0.0.1:9876/mcp`）。
    #[must_use]
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            session_id: None,
        }
    }

    /// 是否已完成 initialize handshake。
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.session_id.is_some()
    }

    /// 產出 `initialize` 請求 body（framing 測試用；真呼叫見 Slice 1/2）。
    #[must_use]
    pub fn initialize_request() -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": { "name": "graphify-plugin-telemetry", "version": "0.1.0" }
            }
        })
    }

    /// 產出 `tools/call` 請求 body（framing 測試用）。
    #[must_use]
    pub fn call_tool_request(id: u64, name: &str, args: &Value) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": name, "arguments": args }
        })
    }

    /// 從 MCP 回應 body 取出 `result.content`（text block 串接）。
    ///
    /// 回應可為純 JSON（`{"result":{"content":[...]}}`）或 SSE
    /// （`event: message\ndata: {...}`）——先剝 SSE 前綴再取 JSON。
    #[must_use]
    pub fn extract_result_content(raw: &str) -> Option<String> {
        let json_str = raw
            .lines()
            .find_map(|l| l.strip_prefix("data: "))
            .unwrap_or(raw);
        let v: Value = serde_json::from_str(json_str).ok()?;
        let content = v.get("result")?.get("content")?.as_array()?;
        let text: Vec<&str> = content
            .iter()
            .filter_map(|c| c.get("text").and_then(Value::as_str))
            .collect();
        if text.is_empty() {
            None
        } else {
            Some(text.join("\n"))
        }
    }

    /// 執行 `tools/call`（streamable HTTP POST）。回傳合併後的 text 內容。
    ///
    /// # Errors
    /// 網路/HTTP 失敗回傳 [`ureq::Error`]；`session_id` 未初始化回傳
    /// [`DracoError::NotInitialized`]。
    pub fn call_tool(&mut self, name: &str, args: &Value) -> Result<String, DracoError> {
        let session = self
            .session_id
            .clone()
            .ok_or(DracoError::NotInitialized)?;
        let body = Self::call_tool_request(2, name, args);
        let resp = ureq::post(&self.base_url)
            .set("Content-Type", "application/json")
            .set("Accept", "application/json, text/event-stream")
            .set("Mcp-Session-Id", &session)
            .timeout(Duration::from_secs(10))
            .send_json(body)?;
        let raw = resp.into_string()?;
        Self::extract_result_content(&raw).ok_or(DracoError::EmptyResult)
    }
}

/// Draco client 錯誤。
#[derive(Debug, thiserror::Error)]
pub enum DracoError {
    #[error("ureq error: {0}")]
    Ureq(Box<ureq::Error>),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("not initialized: call initialize() first")]
    NotInitialized,
    #[error("empty result from Draco")]
    EmptyResult,
}

impl From<ureq::Error> for DracoError {
    fn from(e: ureq::Error) -> Self {
        Self::Ureq(Box::new(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_request_has_expected_shape() {
        let v = DracoMcpClient::initialize_request();
        assert_eq!(v["method"], "initialize");
        assert_eq!(v["params"]["protocolVersion"], "2025-03-26");
        assert_eq!(v["params"]["clientInfo"]["name"], "graphify-plugin-telemetry");
    }

    #[test]
    fn call_tool_request_has_expected_shape() {
        let v = DracoMcpClient::call_tool_request(
            2,
            "fetch_top_hotspots",
            &json!({"workspace_key": "my-app-v1"}),
        );
        assert_eq!(v["method"], "tools/call");
        assert_eq!(v["params"]["name"], "fetch_top_hotspots");
        assert_eq!(v["params"]["arguments"]["workspace_key"], "my-app-v1");
    }

    #[test]
    fn extracts_text_from_plain_json_response() {
        let raw = r#"{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"hello"},{"type":"text","text":"world"}]}}"#;
        assert_eq!(
            DracoMcpClient::extract_result_content(raw).as_deref(),
            Some("hello\nworld")
        );
    }

    #[test]
    fn extracts_text_from_sse_response() {
        let raw = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"sse hit\"}]}}\n\n";
        assert_eq!(
            DracoMcpClient::extract_result_content(raw).as_deref(),
            Some("sse hit")
        );
    }

    #[test]
    fn empty_result_returns_none() {
        let raw = r#"{"jsonrpc":"2.0","id":2,"result":{"content":[]}}"#;
        assert!(DracoMcpClient::extract_result_content(raw).is_none());
    }

    #[test]
    fn not_initialized_call_fails() {
        let mut c = DracoMcpClient::new("http://127.0.0.1:1/mcp");
        let err = c.call_tool("fetch_top_hotspots", &json!({})).unwrap_err();
        assert!(matches!(err, DracoError::NotInitialized));
    }

    #[test]
    fn default_url_respects_env() {
        assert!(default_draco_url().starts_with("http://"));
    }
}
