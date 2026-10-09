use std::{
    borrow::Cow,
    io::{Stdout, Write},
    process::{Command, Stdio},
};

use base64::{Engine, engine::general_purpose::STANDARD};

use crate::{colors::ColorSpec, config::TermColorMapping};

/// Drop control characters (C0, DEL, C1) from user text. Config files are untrusted: a BEL or ESC
/// inside a tab name would otherwise terminate the OSC early and inject arbitrary sequences.
fn sanitize(text: &str) -> Cow<'_, str> {
    if text.chars().any(char::is_control) {
        Cow::Owned(text.chars().filter(|c| !c.is_control()).collect())
    } else {
        Cow::Borrowed(text)
    }
}

const ITERM2_RESET_BG_COLOR_ANSI: &str = "\x1B]6;1;bg;*;default\x07";
const OSC0_RESET_TAB_NAME: &str = "\x1B]0;\x07";

/// WezTerm pane user vars read by the Lua module installed via `term-boss setup wezterm`.
const WEZTERM_VAR_TAB_COLOR: &str = "TAB_COLOR";
const WEZTERM_VAR_TAB_BADGE: &str = "TAB_BADGE";

/// Emit `OSC 1337 ; SetUserVar=name=<base64 value> BEL`.
/// WezTerm requires the value to be standard base64; an empty value clears the var.
fn osc_set_user_var(stdout: &mut Stdout, name: &str, value: &str) {
    let encoded = STANDARD.encode(value.as_bytes());
    let _ = write!(stdout, "\x1B]1337;SetUserVar={}={}\x07", name, encoded);
}

/// Run `wezterm cli <args>` for the current pane (wezterm reads WEZTERM_PANE from the env).
/// Output is discarded so nothing leaks into the shell on every `cd`.
fn wezterm_cli(args: &[&str]) {
    let _ = Command::new("wezterm")
        .arg("cli")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[derive(Debug, PartialEq, Eq, Default)]
pub enum TerminalApp {
    #[default]
    Unknown,
    Iterm2,
    Alacritty,
    Wezterm,
    Konsole,
}

/// Dynamic pane colours set via the xterm OSC 10/11/12 family.
/// Reset uses the matching OSC 110/111/112.
#[derive(Debug, Clone, Copy)]
pub enum PaneColor {
    Foreground = 10,
    Background = 11,
    Cursor = 12,
}

impl TerminalApp {
    pub fn reset_tab_color(&self, stdout: &mut Stdout) {
        match self {
            TerminalApp::Iterm2 => {
                let _ = stdout.write_all(ITERM2_RESET_BG_COLOR_ANSI.as_bytes());
            }
            TerminalApp::Wezterm => {
                osc_set_user_var(stdout, WEZTERM_VAR_TAB_COLOR, "");
            }
            TerminalApp::Alacritty | TerminalApp::Konsole | TerminalApp::Unknown => { /* no op */ }
        }
    }

    pub fn reset_tab_name(&self, stdout: &mut Stdout) {
        match self {
            TerminalApp::Iterm2 | TerminalApp::Alacritty | TerminalApp::Konsole => {
                let _ = stdout.write_all(OSC0_RESET_TAB_NAME.as_bytes());
            }
            TerminalApp::Wezterm => {
                // An empty tab title makes wezterm fall back to the active pane's title.
                wezterm_cli(&["set-tab-title", ""]);
            }
            TerminalApp::Unknown => { /* no op */ }
        }
    }

    pub fn set_badge_text(&self, stdout: &mut Stdout, badgetext: &str) {
        let badgetext = &*sanitize(badgetext);
        match self {
            TerminalApp::Iterm2 => {
                let encoded = STANDARD.encode(badgetext.as_bytes());
                let _ = write!(stdout, "\x1B]1337;SetBadgeFormat={}\x07", encoded);
            }
            TerminalApp::Wezterm => {
                // Shown in the right status area by termboss.lua; empty clears it.
                osc_set_user_var(stdout, WEZTERM_VAR_TAB_BADGE, badgetext);
            }
            TerminalApp::Alacritty | TerminalApp::Konsole | TerminalApp::Unknown => { /* no op */ }
        }
    }

    pub fn set_tab_bg_color(&self, stdout: &mut Stdout, tabcolor: &ColorSpec) {
        match self {
            TerminalApp::Iterm2 => {
                let [r, g, b] = tabcolor.get_rgb();
                let _ = write!(stdout, "\x1B]6;1;bg;red;brightness;{}\x07", r);
                let _ = write!(stdout, "\x1B]6;1;bg;green;brightness;{}\x07", g);
                let _ = write!(stdout, "\x1B]6;1;bg;blue;brightness;{}\x07", b);
            }
            TerminalApp::Wezterm => {
                osc_set_user_var(stdout, WEZTERM_VAR_TAB_COLOR, &format!("#{:x}", tabcolor));
            }
            TerminalApp::Alacritty | TerminalApp::Konsole | TerminalApp::Unknown => { /* no op */ }
        }
    }

    pub fn set_tab_name(&self, stdout: &mut Stdout, tabname: &str) {
        let tabname = &*sanitize(tabname);
        match self {
            TerminalApp::Iterm2 | TerminalApp::Alacritty | TerminalApp::Konsole => {
                let _ = write!(stdout, "\x1B]0;{}\x07", tabname);
            }
            TerminalApp::Wezterm => {
                wezterm_cli(&["set-tab-title", tabname]);
            }
            TerminalApp::Unknown => { /* no op */ }
        }
    }

    /// Set a pane colour (foreground/background/cursor). Same escape sequence on every terminal.
    pub fn set_pane_color(&self, stdout: &mut Stdout, which: PaneColor, color: &ColorSpec) {
        if *self == TerminalApp::Unknown {
            return;
        }
        let _ = write!(stdout, "\x1B]{};#{:x}\x07", which as u8, color);
    }

    /// Reset a pane colour to the terminal's scheme default (OSC 110/111/112).
    pub fn reset_pane_color(&self, stdout: &mut Stdout, which: PaneColor) {
        if *self == TerminalApp::Unknown {
            return;
        }
        let _ = write!(stdout, "\x1B]{}\x07", which as u8 + 100);
    }

    /// Set one ANSI palette slot (OSC 4). Same escape sequence on every terminal.
    pub fn set_palette_color(&self, stdout: &mut Stdout, slot: TermColorMapping, color: &ColorSpec) {
        if *self == TerminalApp::Unknown {
            return;
        }
        let _ = write!(stdout, "\x1B]4;{};#{:x}\x07", slot as u8, color);
    }

    /// Reset the given palette slots to the terminal's defaults in one OSC 104 sequence.
    pub fn reset_palette_colors(&self, stdout: &mut Stdout, slots: impl Iterator<Item = TermColorMapping>) {
        if *self == TerminalApp::Unknown {
            return;
        }
        let mut seq = String::from("\x1B]104");
        for slot in slots {
            seq.push(';');
            seq.push_str(&(slot as u8).to_string());
        }
        if seq.len() > 5 {
            seq.push('\x07');
            let _ = stdout.write_all(seq.as_bytes());
        }
    }
}
