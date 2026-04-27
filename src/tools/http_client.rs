use cortex_sdk::{Tool, ToolCapabilities, ToolError, ToolResult};
use std::process::Command;

/// Enhanced HTTP client for API testing and interaction.
/// Uses curl under the hood — available on virtually every system.
pub struct HttpRequestTool;

impl Tool for HttpRequestTool {
    fn name(&self) -> &'static str {
        "http"
    }

    fn description(&self) -> &'static str {
        "Make an HTTP request (GET, POST, PUT, DELETE, PATCH).\n\n\
         Use for API testing, webhook debugging, or fetching structured data. \
         More capable than web_fetch: supports all methods, custom headers, \
         request bodies, and returns status codes."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "Request URL" },
                "method": {
                    "type": "string",
                    "description": "HTTP method (default: GET)",
                    "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD"]
                },
                "headers": {
                    "type": "object",
                    "description": "Request headers as key-value pairs"
                },
                "body": {
                    "type": "string",
                    "description": "Request body (for POST/PUT/PATCH)"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default: 30)"
                }
            },
            "required": ["url"]
        })
    }

    fn capabilities(&self) -> ToolCapabilities {
        super::caps([super::network_effect("requested URL")])
    }

    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let url = input
            .get("url")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("missing 'url'".into()))?;
        let method = input
            .get("method")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("GET");
        let timeout = input
            .get("timeout")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(30);

        let mut args = vec![
            "-s".to_string(),
            "-w".to_string(),
            "\n---HTTP_STATUS:%{http_code}---".to_string(),
            "-X".to_string(),
            method.to_string(),
            "--max-time".to_string(),
            timeout.to_string(),
        ];

        // Headers
        if let Some(headers) = input.get("headers").and_then(serde_json::Value::as_object) {
            for (k, v) in headers {
                if let Some(val) = v.as_str() {
                    args.push("-H".to_string());
                    args.push(format!("{k}: {val}"));
                }
            }
        }

        // Body
        if let Some(body) = input.get("body").and_then(serde_json::Value::as_str) {
            args.push("-d".to_string());
            args.push(body.to_string());
            // Auto-set content-type if not explicitly provided
            let has_content_type = input
                .get("headers")
                .and_then(serde_json::Value::as_object)
                .is_some_and(|h| h.keys().any(|k| k.to_lowercase() == "content-type"));
            if !has_content_type {
                args.push("-H".to_string());
                args.push("Content-Type: application/json".to_string());
            }
        }

        args.push(url.to_string());

        let output = Command::new("curl")
            .args(&args)
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("curl failed: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() && stdout.is_empty() {
            return Err(ToolError::ExecutionFailed(format!(
                "request failed: {}",
                stderr.trim()
            )));
        }

        // Parse out status code from our -w format
        let result = stdout.rfind("---HTTP_STATUS:").map_or_else(
            || stdout.to_string(),
            |idx| {
                let status = &stdout[idx + 15..stdout.len() - 3];
                let body = stdout[..idx].trim();
                format!("HTTP {status}\n\n{body}")
            },
        );

        Ok(ToolResult::success(result))
    }
}
