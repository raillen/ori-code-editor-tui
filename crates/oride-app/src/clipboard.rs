//! Clipboard: arboard (sistema) com fallback em buffer interno.

use std::sync::Mutex;

static INTERNAL: Mutex<String> = Mutex::new(String::new());

/// Copia texto para o clipboard do sistema; se falhar, usa buffer interno.
pub fn copy_text(text: &str) -> Result<(), String> {
    if let Ok(mut guard) = INTERNAL.lock() {
        *guard = text.to_string();
    }
    match arboard::Clipboard::new() {
        Ok(mut cb) => cb
            .set_text(text.to_string())
            .map_err(|e| format!("clipboard: {e}")),
        Err(e) => {
            // SSH / headless: mantém interno
            let _ = e;
            Ok(())
        }
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
    INTERNAL.lock().map(|g| g.clone()).unwrap_or_default()
}
