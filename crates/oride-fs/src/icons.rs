//! Ícones de arquivo (Nerd Font glyphs; fallback ASCII).

/// Retorna glyph para path. `use_nerd` escolhe Nerd Font vs ASCII.
#[must_use]
pub fn file_icon(
    path: &std::path::Path,
    is_dir: bool,
    expanded: bool,
    use_nerd: bool,
) -> &'static str {
    if is_dir {
        return if use_nerd {
            if expanded {
                "" // nf-fa-folder_open
            } else {
                "" // nf-fa-folder
            }
        } else if expanded {
            "[-]"
        } else {
            "[+]"
        };
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if !use_nerd {
        return match ext.as_str() {
            "oris" => ".o",
            "rs" => ".r",
            "md" => ".m",
            "toml" => ".t",
            "json" => ".j",
            "html" | "htm" => ".h",
            "css" => ".c",
            "js" | "mjs" | "ts" => ".s",
            _ => " .",
        };
    }

    match ext.as_str() {
        "oris" => "", // generic code
        "rs" => "",
        "md" => "",
        "toml" => "",
        "json" => "",
        "html" | "htm" => "",
        "css" => "",
        "js" | "mjs" => "",
        "ts" => "",
        "py" => "",
        "sh" | "bash" | "zsh" => "",
        "lock" => "",
        "gitignore" => "",
        _ => "",
    }
}
