//! Restaura o terminal ao sair (raw mode + alternate screen).

use std::io::{self, Stdout, Write};

use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, supports_keyboard_enhancement, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

pub struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    keyboard_enhanced: bool,
    mouse_captured: bool,
}

impl TerminalGuard {
    /// `enable_mouse`: se true, captura mouse no enter (default do produto: false).
    pub fn enter(enable_mouse: bool) -> anyhow::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        if enable_mouse {
            execute!(stdout, EnableMouseCapture)?;
        }

        // Kitty/WezTerm/Foot/Ghostty: reporta Ctrl+Shift+letra corretamente.
        // Sem isso, Ctrl+Shift+S vira Ctrl+S e o Save As nunca dispara.
        let keyboard_enhanced = if supports_keyboard_enhancement().unwrap_or(false) {
            execute!(
                stdout,
                PushKeyboardEnhancementFlags(
                    KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                        | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                        | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
                )
            )
            .is_ok()
        } else {
            // Tenta mesmo assim — alguns emuladores respondem ao push sem o query.
            execute!(
                stdout,
                PushKeyboardEnhancementFlags(
                    KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                        | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
                )
            )
            .is_ok()
        };

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self {
            terminal,
            keyboard_enhanced,
            mouse_captured: enable_mouse,
        })
    }

    pub fn terminal(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }

    /// Liga/desliga captura de mouse em runtime (menu / config).
    pub fn set_mouse_capture(&mut self, enabled: bool) {
        if enabled == self.mouse_captured {
            return;
        }
        let backend = self.terminal.backend_mut();
        if enabled {
            let _ = execute!(backend, EnableMouseCapture);
        } else {
            let _ = execute!(backend, DisableMouseCapture);
        }
        let _ = backend.flush();
        self.mouse_captured = enabled;
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.keyboard_enhanced {
            let _ = execute!(self.terminal.backend_mut(), PopKeyboardEnhancementFlags);
        }
        let _ = disable_raw_mode();
        if self.mouse_captured {
            let _ = execute!(self.terminal.backend_mut(), DisableMouseCapture);
        }
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}
