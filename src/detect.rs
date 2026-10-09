use std::env;

use crate::terminal_app::TerminalApp;



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

    if term_app == TerminalApp::Unknown
        && env::var("TERM").is_ok_and(|term| term.contains("alacritty"))
    {
        term_app = TerminalApp::Alacritty;
    }

    if term_app == TerminalApp::Unknown {
        if env::var("KONSOLE_VERSION").is_ok() {
            term_app = TerminalApp::Konsole;
        } else if env::var("WEZTERM_PANE").is_ok() {
            term_app = TerminalApp::Wezterm;
        } else if env::var("ITERM_SESSION_ID").is_ok() {
            term_app = TerminalApp::Iterm2;
        }
    }

    term_app
}