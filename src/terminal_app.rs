use std::{
    borrow::Cow,
    env,
    io::{ErrorKind, Stdout, Write},
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

/// Detach a helper process from the terminal so nothing leaks into the shell on every `cd`.
fn quiet(mut cmd: Command) -> Command {
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    cmd
}

/// Run `wezterm cli <args>` for the current pane (wezterm reads WEZTERM_PANE from the env).
fn wezterm_cli(args: &[&str]) {
    let mut cmd = Command::new("wezterm");
    cmd.arg("cli").args(args);
    let _ = quiet(cmd).status();
}

/// Konsole's per-session D-Bus interface. Konsole exports the bus name and object path of the
/// session that owns this shell as KONSOLE_DBUS_SERVICE / KONSOLE_DBUS_SESSION.
const KONSOLE_DBUS_IFACE: &str = "org.kde.konsole.Session";
/// Konsole's built-in default tab title format, restored by `reset_tab_name`.
const KONSOLE_DEFAULT_TAB_FORMAT: &str = "%d : %n";

enum DbusArg<'a> {
    Str(&'a str),
    Bool(bool),
}

/// Whether Konsole's D-Bus interface actually works, not just whether the env coordinates are
/// set: makes a harmless, read-only call (`sessionId`) and reports its success. A missing
/// `dbus-send`/`busctl`, or Konsole rejecting the call, counts as unavailable.
pub fn konsole_dbus_available() -> bool {
    konsole_dbus("sessionId", &[])
}

/// Call `method` on the Konsole session that owns this shell. Uses `dbus-send` (ships with D-Bus)
/// and falls back to `busctl` (systemd) if that is missing. Returns whether the call succeeded;
/// `false` without the env vars, without a working D-Bus client, or if the call itself fails.
fn konsole_dbus(method: &str, args: &[DbusArg]) -> bool {
    let (Ok(service), Ok(path)) = (env::var("KONSOLE_DBUS_SERVICE"), env::var("KONSOLE_DBUS_SESSION")) else {
        return false;
    };

    let mut cmd = Command::new("dbus-send");
    cmd.args(["--session", "--type=method_call", &format!("--dest={service}"), &path, &format!("{KONSOLE_DBUS_IFACE}.{method}")]);
    for arg in args {
        cmd.arg(match arg {
            DbusArg::Str(s) => format!("string:{s}"),
            DbusArg::Bool(b) => format!("boolean:{b}"),
        });
    }
    match quiet(cmd).status() {
        Ok(status) => return status.success(),
        Err(e) if e.kind() == ErrorKind::NotFound => { /* try busctl */ }
        Err(_) => return false,
    }

    let signature: String = args
        .iter()
        .map(|arg| match arg {
            DbusArg::Str(_) => 's',
            DbusArg::Bool(_) => 'b',
        })
        .collect();
    let mut cmd = Command::new("busctl");
    cmd.args(["--user", "call", &service, &path, KONSOLE_DBUS_IFACE, method, &signature]);
    for arg in args {
        cmd.arg(match arg {
            DbusArg::Str(s) => (*s).to_string(),
            DbusArg::Bool(b) => b.to_string(),
        });
    }
    quiet(cmd).status().map(|s| s.success()).unwrap_or(false)
}

/// The emit methods below write for every variant, including `Unknown`: deciding whether to
/// emit at all is the caller's job (`term-boss set` exits early for `Unknown`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerminalApp {
    #[default]
    Unknown,
    Iterm2,
    Alacritty,
    Wezterm,
    Konsole,
    /// VS Code's integrated terminal: standard escapes. The tab shows the OSC 0 title only when
    /// `terminal.integrated.tabs.title` is `${sequence}` (see `term-boss setup vscode`).
    Vscode,
    /// Ghostty: standard escapes. It has no escape for tab colour (the macOS colour picker is
    /// UI-only) or badge (OSC 1337 is parsed but not implemented), so the badge goes in the title.
    Ghostty,
    /// Any other real terminal: standard escapes only (OSC 0, 4, 10, 11, 12 and their resets).
    Generic,
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
    /// Can this terminal show a tab title at all?
    pub fn supports_tab_name(&self) -> bool {
        !matches!(self, TerminalApp::Unknown)
    }

    /// Can this terminal show badge text somewhere other than the tab title? Konsole can only
    /// via D-Bus, so it counts only when Konsole exported the bus coordinates.
    pub fn supports_badge(&self) -> bool {
        match self {
            TerminalApp::Iterm2 | TerminalApp::Wezterm => true,
            TerminalApp::Konsole => konsole_dbus_available(),
            TerminalApp::Alacritty | TerminalApp::Vscode | TerminalApp::Ghostty | TerminalApp::Generic | TerminalApp::Unknown => false,
        }
    }

    /// A terminal with tabs but no badge shows the badge in the tab title instead.
    pub fn badge_in_title(&self, badgetext: Option<&str>) -> bool {
        badgetext.is_some() && self.supports_tab_name() && !self.supports_badge()
    }

    /// The tab title `set` sends: the configured name, with the badge folded in as
    /// `name [badge]` (or just `[badge]`) when `badge_in_title`.
    pub fn effective_title(&self, tabname: Option<&str>, badgetext: Option<&str>) -> Option<String> {
        let badge = if self.badge_in_title(badgetext) { badgetext } else { None };
        match (tabname, badge) {
            (Some(name), Some(badge)) => Some(format!("{name} [{badge}]")),
            (None, Some(badge)) => Some(format!("[{badge}]")),
            (name, None) => name.map(str::to_owned),
        }
    }

    pub fn reset_tab_color(&self, stdout: &mut Stdout) {
        match self {
            TerminalApp::Iterm2 => {
                let _ = stdout.write_all(ITERM2_RESET_BG_COLOR_ANSI.as_bytes());
            }
            TerminalApp::Wezterm => {
                osc_set_user_var(stdout, WEZTERM_VAR_TAB_COLOR, "");
            }
            TerminalApp::Konsole => {
                // OSC 34 ignores an empty/invalid colour, so clearing needs D-Bus: an empty
                // string becomes an invalid QColor, which is Konsole's "no tab colour".
                konsole_dbus("setTabColor", &[DbusArg::Str("")]);
            }
            TerminalApp::Alacritty | TerminalApp::Vscode | TerminalApp::Ghostty | TerminalApp::Generic | TerminalApp::Unknown => { /* no op */ }
        }
    }

    pub fn reset_tab_name(&self, stdout: &mut Stdout) {
        match self {
            TerminalApp::Iterm2 | TerminalApp::Alacritty | TerminalApp::Vscode | TerminalApp::Ghostty | TerminalApp::Generic => {
                let _ = stdout.write_all(OSC0_RESET_TAB_NAME.as_bytes());
            }
            TerminalApp::Konsole => {
                // Restore Konsole's default title format (a profile with a custom format gets
                // the built-in default instead; the format is not readable from the shell).
                let _ = write!(stdout, "\x1B]30;{}\x07", KONSOLE_DEFAULT_TAB_FORMAT);
            }
            TerminalApp::Wezterm => {
                // An empty tab title makes wezterm fall back to the active pane's title.
                wezterm_cli(&["set-tab-title", "--", ""]);
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
            TerminalApp::Konsole => {
                // Konsole's badge (an overlay in the view, like iTerm2's) is D-Bus only.
                if badgetext.is_empty() {
                    konsole_dbus("setBadgeEnabled", &[DbusArg::Bool(false)]);
                } else {
                    konsole_dbus("setBadgeText", &[DbusArg::Str(badgetext)]);
                    konsole_dbus("setBadgeEnabled", &[DbusArg::Bool(true)]);
                }
            }
            TerminalApp::Alacritty | TerminalApp::Vscode | TerminalApp::Ghostty | TerminalApp::Generic | TerminalApp::Unknown => { /* no op */ }
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
            TerminalApp::Konsole => {
                // Konsole-specific OSC 34 "session colour" = tab colour.
                let _ = write!(stdout, "\x1B]34;#{:x}\x07", tabcolor);
            }
            TerminalApp::Alacritty | TerminalApp::Vscode | TerminalApp::Ghostty | TerminalApp::Generic | TerminalApp::Unknown => { /* no op */ }
        }
    }

    pub fn set_tab_name(&self, stdout: &mut Stdout, tabname: &str) {
        let tabname = &*sanitize(tabname);
        match self {
            TerminalApp::Iterm2 | TerminalApp::Alacritty | TerminalApp::Vscode | TerminalApp::Ghostty | TerminalApp::Generic => {
                let _ = write!(stdout, "\x1B]0;{}\x07", tabname);
            }
            TerminalApp::Konsole => {
                // Konsole-specific OSC 30 sets the tab title directly. OSC 0 only sets the window
                // title, which the default tab format ("%d : %n") does not show.
                let _ = write!(stdout, "\x1B]30;{}\x07", tabname);
            }
            TerminalApp::Wezterm => {
                // `--` so a title starting with '-' is not parsed as a flag.
                wezterm_cli(&["set-tab-title", "--", tabname]);
            }
            TerminalApp::Unknown => { /* no op */ }
        }
    }

    /// Set a pane colour (foreground/background/cursor). Same escape sequence on every terminal.
    pub fn set_pane_color(&self, stdout: &mut Stdout, which: PaneColor, color: &ColorSpec) {
        let _ = write!(stdout, "\x1B]{};#{:x}\x07", which as u8, color);
    }

    /// Reset a pane colour to the terminal's scheme default (OSC 110/111/112).
    pub fn reset_pane_color(&self, stdout: &mut Stdout, which: PaneColor) {
        let _ = write!(stdout, "\x1B]{}\x07", which as u8 + 100);
    }

    /// Set one ANSI palette slot (OSC 4). Same escape sequence on every terminal.
    pub fn set_palette_color(&self, stdout: &mut Stdout, slot: TermColorMapping, color: &ColorSpec) {
        let _ = write!(stdout, "\x1B]4;{};#{:x}\x07", slot as u8, color);
    }

    /// Reset the given palette slots to the terminal's defaults in one OSC 104 sequence.
    pub fn reset_palette_colors(&self, stdout: &mut Stdout, slots: impl Iterator<Item = TermColorMapping>) {
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
