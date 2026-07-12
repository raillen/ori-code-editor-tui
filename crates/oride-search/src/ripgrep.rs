//! Backend `rg` (ripgrep).

use std::path::Path;
use std::process::Command;

use crate::{SearchError, SearchHit, SearchQuery};

/// `true` se `rg` responde a `--version`.
#[must_use]
pub fn rg_available() -> bool {
    Command::new("rg")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Busca via `rg --json` (ou fallback `--vimgrep` se JSON falhar).
pub fn search_with_rg(
    root: &Path,
    query: &SearchQuery,
    max_hits: usize,
) -> Result<Vec<SearchHit>, SearchError> {
    let mut cmd = Command::new("rg");
    cmd.current_dir(root);
    cmd.arg("--json");
    cmd.arg("--line-number");
    cmd.arg("--column");
    cmd.arg("--no-heading");
    cmd.arg("--color=never");
    cmd.arg("--hidden");
    cmd.arg("--glob").arg("!target/**");
    cmd.arg("--glob").arg("!node_modules/**");
    cmd.arg("--glob").arg("!.git/**");
    if !query.case_sensitive {
        cmd.arg("-i");
    }
    if !query.use_regex {
        cmd.arg("-F");
    }
    cmd.arg("--max-count").arg("50"); // por arquivo
    cmd.arg(&query.pattern);
    cmd.arg(".");

    let output = cmd
        .output()
        .map_err(|e| SearchError::Other(format!("rg spawn: {e}")))?;

    // rg exit 1 = no matches; 0 = ok; 2 = error
    if !output.status.success() && output.status.code() != Some(1) {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(SearchError::Other(format!(
            "rg failed ({}): {err}",
            output.status
        )));
    }

    parse_rg_json(&output.stdout, max_hits)
}

fn parse_rg_json(stdout: &[u8], max_hits: usize) -> Result<Vec<SearchHit>, SearchError> {
    let text = String::from_utf8_lossy(stdout);
    let mut hits = Vec::new();
    for line in text.lines() {
        if hits.len() >= max_hits {
            break;
        }
        let Ok(v) = serde_json_lite_match(line) else {
            continue;
        };
        if let Some(hit) = v {
            hits.push(hit);
        }
    }
    Ok(hits)
}

/// Parse mínimo de uma linha `--json` do rg (tipo match).
fn serde_json_lite_match(line: &str) -> Result<Option<SearchHit>, SearchError> {
    // Evita depender de serde_json no crate se não for necessário —
    // mas workspace já tem serde_json; usar para robustez.
    let v: serde_json::Value =
        serde_json::from_str(line).map_err(|e| SearchError::Other(format!("rg json: {e}")))?;
    if v.get("type").and_then(|t| t.as_str()) != Some("match") {
        return Ok(None);
    }
    let data = v
        .get("data")
        .ok_or_else(|| SearchError::Other("rg match without data".into()))?;
    let path = data
        .get("path")
        .and_then(|p| p.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| SearchError::Other("rg path".into()))?;
    let line_num = data
        .get("line_number")
        .and_then(|n| n.as_u64())
        .unwrap_or(1) as usize;
    let line_text = data
        .get("lines")
        .and_then(|l| l.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .trim_end_matches(['\n', '\r'])
        .to_string();
    let column = data
        .get("submatches")
        .and_then(|s| s.as_array())
        .and_then(|a| a.first())
        .and_then(|m| m.get("start"))
        .and_then(|s| s.as_u64())
        .map(|c| c as usize + 1)
        .unwrap_or(1);

    Ok(Some(SearchHit {
        path: Path::new(path).to_path_buf(),
        line: line_num.max(1),
        column: column.max(1),
        line_text,
    }))
}
