//! Leitura mínima de `.editorconfig` (indent_style / indent_size).

use std::fs;
use std::path::{Path, PathBuf};

/// Preferências de indentação resolvidas a partir de `.editorconfig`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorIndent {
    pub tab_size: u8,
    pub insert_spaces: bool,
}

/// Procura `.editorconfig` de `file` subindo diretórios e aplica seções
/// `[*]` e `[*.ext]` (e globs simples `*.rs`).
#[must_use]
pub fn resolve_indent_for_file(file: &Path, fallback: EditorIndent) -> EditorIndent {
    let mut result = fallback;
    let ext = file
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let name = file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    for dir in ancestors_of(file) {
        let cfg_path = dir.join(".editorconfig");
        if !cfg_path.is_file() {
            continue;
        }
        if let Ok(text) = fs::read_to_string(&cfg_path) {
            apply_editorconfig(&text, &ext, &name, &mut result);
            // root = true interrompe a subida
            if text.lines().any(|l| {
                let t = l.trim();
                t.eq_ignore_ascii_case("root=true") || t.eq_ignore_ascii_case("root = true")
            }) {
                break;
            }
        }
    }
    result
}

fn ancestors_of(file: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let start = file.parent().unwrap_or(file);
    let mut cur = Some(start.to_path_buf());
    while let Some(dir) = cur {
        out.push(dir.clone());
        cur = dir.parent().map(Path::to_path_buf);
        if out.len() > 32 {
            break;
        }
    }
    out
}

fn apply_editorconfig(text: &str, ext: &str, filename: &str, out: &mut EditorIndent) {
    let mut current_applies = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let pattern = &line[1..line.len() - 1];
            current_applies = section_matches(pattern, ext, filename);
            continue;
        }
        if !current_applies {
            // propriedades globais (root=) ignoradas aqui
            if !line.contains('=') {
                continue;
            }
            // só aplica se ainda não entramos em seção — algumas configs
            // põem root= no topo; indent sempre sob seção
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim().to_ascii_lowercase();
        let v = v.trim().to_ascii_lowercase();
        match k.as_str() {
            "indent_style" => {
                if v == "space" {
                    out.insert_spaces = true;
                } else if v == "tab" {
                    out.insert_spaces = false;
                }
            }
            "indent_size" => {
                if v == "tab" {
                    // usa tab_width se existir; senão mantém
                } else if let Ok(n) = v.parse::<u8>() {
                    out.tab_size = n.max(1);
                }
            }
            "tab_width" => {
                if let Ok(n) = v.parse::<u8>() {
                    if !out.insert_spaces {
                        out.tab_size = n.max(1);
                    }
                }
            }
            _ => {}
        }
    }
}

fn section_matches(pattern: &str, ext: &str, filename: &str) -> bool {
    let p = pattern.trim();
    if p == "*" {
        return true;
    }
    // *.rs / **/*.rs
    if let Some(rest) = p.strip_prefix("*.") {
        return ext == rest.to_ascii_lowercase();
    }
    if let Some(rest) = p.strip_prefix("**/*.") {
        return ext == rest.to_ascii_lowercase();
    }
    // nome exato
    p.eq_ignore_ascii_case(filename)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn reads_indent_from_editorconfig() {
        let dir = tempfile::tempdir().unwrap();
        let mut f = fs::File::create(dir.path().join(".editorconfig")).unwrap();
        writeln!(
            f,
            "root = true\n\n[*]\nindent_style = space\nindent_size = 2\n\n[*.rs]\nindent_size = 4\n"
        )
        .unwrap();
        let file = dir.path().join("main.rs");
        fs::write(&file, "fn main() {}\n").unwrap();
        let ind = resolve_indent_for_file(
            &file,
            EditorIndent {
                tab_size: 8,
                insert_spaces: false,
            },
        );
        assert!(ind.insert_spaces);
        assert_eq!(ind.tab_size, 4);
    }
}
