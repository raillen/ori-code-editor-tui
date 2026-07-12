//! Framing Content-Length e helpers JSON-RPC.

use std::io::{Read, Write};

use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("protocol: {0}")]
    Protocol(String),
}

pub fn write_message(w: &mut dyn Write, body: &Value) -> Result<(), ProtocolError> {
    let bytes = serde_json::to_vec(body)?;
    write!(w, "Content-Length: {}\r\n\r\n", bytes.len())?;
    w.write_all(&bytes)?;
    w.flush()?;
    Ok(())
}

pub fn read_message(r: &mut dyn Read) -> Result<Value, ProtocolError> {
    let mut headers = Vec::new();
    let mut buf = [0u8; 1];
    // lê até \r\n\r\n
    loop {
        let n = r.read(&mut buf)?;
        if n == 0 {
            return Err(ProtocolError::Protocol("eof before headers".into()));
        }
        headers.push(buf[0]);
        if headers.len() >= 4 && headers[headers.len() - 4..] == *b"\r\n\r\n" {
            break;
        }
        if headers.len() > 64 * 1024 {
            return Err(ProtocolError::Protocol("headers too large".into()));
        }
    }
    let header_str = String::from_utf8_lossy(&headers);
    let mut content_length = None;
    for line in header_str.lines() {
        let line = line.trim();
        if let Some(rest) = line
            .strip_prefix("Content-Length:")
            .or_else(|| line.strip_prefix("content-length:"))
        {
            content_length = rest.trim().parse::<usize>().ok();
        }
    }
    let len =
        content_length.ok_or_else(|| ProtocolError::Protocol("missing Content-Length".into()))?;
    let mut body = vec![0u8; len];
    r.read_exact(&mut body)?;
    Ok(serde_json::from_slice(&body)?)
}

pub fn request(id: u64, method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    })
}

pub fn notification(method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
    })
}

pub fn path_to_uri(path: &std::path::Path) -> String {
    let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    format!("file://{}", abs.display())
}
