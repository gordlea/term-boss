use std::env;

use crate::terminal_app::TerminalApp;

/// Environment variables shown by `term-boss show`: the ones `detect_terminal` consults, plus the
/// Konsole D-Bus coordinates its badge and tab-colour reset need.
pub const DETECTION_VARS: [&str; 8] = [
    "TERM_PROGRAM",
    "TERM",
    "KONSOLE_VERSION",
    "ALACRITTY_WINDOW_ID",
    "WEZTERM_PANE",
    "ITERM_SESSION_ID",
    "KONSOLE_DBUS_SERVICE",
    "KONSOLE_DBUS_SESSION",
];

pub fn detection_evidence() -> Vec<(&'static str, Option<String>)> {
    DETECTION_VARS.iter().map(|var| (*var, env::var(var).ok())).collect()
}



pub fn detect_terminal() -> TerminalApp {
    let mut term_app = if let Ok(term_program) = env::var("TERM_PROGRAM") {
        match term_program.as_str() {
            "iTerm.app" => TerminalApp::Iterm2,
            "WezTerm" => TerminalApp::Wezterm,
            _ => TerminalApp::Unknown,
        }
    } else {
        TerminalApp::Unknown
    };

    if term_app == TerminalApp::Unknown {
        // Alacritty is checked before KONSOLE_VERSION: Konsole's variable is inherited by every
        // child process, so an Alacritty started from a Konsole shell has both, and only the
        // Alacritty ones describe the terminal actually rendering this shell. ALACRITTY_WINDOW_ID
        // / ALACRITTY_SOCKET are exported regardless of the configured TERM, which is often
        // "xterm-256color" rather than "alacritty".
        if env::var("TERM").is_ok_and(|term| term.contains("alacritty"))
            || env::var_os("ALACRITTY_WINDOW_ID").is_some()
            || env::var_os("ALACRITTY_SOCKET").is_some()
        {
            term_app = TerminalApp::Alacritty;
        } else if env::var("KONSOLE_VERSION").is_ok() {
            term_app = TerminalApp::Konsole;
        } else if env::var("WEZTERM_PANE").is_ok() {
            term_app = TerminalApp::Wezterm;
        } else if env::var("ITERM_SESSION_ID").is_ok() {
            term_app = TerminalApp::Iterm2;
        }
    }

    if term_app == TerminalApp::Unknown {
        // Anything else that looks like a real terminal gets the standard escapes. TERM unset
        // or "dumb" (scripts, Emacs shells, CI) means nobody would render them.
        let term = env::var("TERM").unwrap_or_default();
        if !term.is_empty() && term != "dumb" {
            term_app = TerminalApp::Generic;
        }
    }

    term_app
}