//! Clipboard: arboard (sistema) + OSC52 (SSH) + fallback interno.

use std::io::{self, Write};
use std::sync::Mutex;

static INTERNAL: Mutex<String> = Mutex::new(String::new());

/// Copia texto para o clipboard do sistema; se falhar, usa buffer interno + OSC52.
pub fn copy_text(text: &str) -> Result<(), String> {
    if let Ok(mut guard) = INTERNAL.lock() {
        *guard = text.to_string();
    }
    // OSC52: funciona em muitos terminais via SSH
    let _ = write_osc52(text);
    match arboard::Clipboard::new() {
        Ok(mut cb) => cb
            .set_text(text.to_string())
            .map_err(|e| format!("clipboard: {e}")),
        Err(_) => Ok(()),
    }
}

/// Lê clipboard do sistema; se vazio/falhar, usa buffer interno.
#[must_use]
pub fn paste_text() -> String {
    if let Ok(mut cb) = arboard::Clipboard::new() {
        if let Ok(t) = cb.get_text() {
            if !t.is_empty() {
                return t;
            }
        }
    }
    internal_text()
}

/// Só o buffer interno (testes / fallback sem X11).
#[must_use]
pub fn internal_text() -> String {
    INTERNAL.lock().map(|g| g.clone()).unwrap_or_default()
}

fn write_osc52(text: &str) -> io::Result<()> {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
    // OSC 52: ESC ] 52 ; c ; <base64> BEL
    let seq = format!("\x1b]52;c;{b64}\x07");
    let mut out = io::stdout();
    out.write_all(seq.as_bytes())?;
    out.flush()?;
    Ok(())
}
