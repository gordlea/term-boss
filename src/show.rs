use std::{
    env,
    io::{Stdout, Write},
    path::PathBuf,
};

use anyhow::Result;

use crate::{
    config::{self, SchemeOrigin, SetConfig, Source},
    detect, expand,
    setup_vscode::{self, SettingsState, TITLE_KEY, TITLE_VALUE},
    setup_wezterm,
    terminal_app::{self, TerminalApp},
};

/// Explain what `term-boss set` would do from the current directory, without doing it.
pub fn run(cli: &SetConfig, stdout: &mut Stdout) -> Result<()> {
    let terminal = detect::detect_terminal();
    match &terminal {
        TerminalApp::Unknown => {
            writeln!(stdout, "Terminal: Unknown (TERM unset or \"dumb\"; term-boss set would exit without output)")?
        }
        TerminalApp::Vscode => writeln!(
            stdout,
            "Terminal: Vscode (VS Code integrated terminal: title, pane colours, palette; no tab colour or badge)"
        )?,
        TerminalApp::Ghostty => writeln!(
            stdout,
            "Terminal: Ghostty (title, pane colours, palette; no tab colour or badge escape)"
        )?,
        TerminalApp::Generic => writeln!(
            stdout,
            "Terminal: Generic (unrecognised; standard escapes only: title, pane colours, palette; no tab colour or badge)"
        )?,
        terminal => writeln!(stdout, "Terminal: {terminal:?}")?,
    }
    for (var, value) in detect::detection_evidence() {
        writeln!(stdout, "  {var}={}", value.as_deref().unwrap_or("(unset)"))?;
    }
    if terminal == TerminalApp::Konsole {
        if terminal_app::konsole_dbus_available() {
            writeln!(stdout, "  D-Bus (badge, tab-colour reset): available")?;
        } else {
            writeln!(
                stdout,
                "  D-Bus (badge, tab-colour reset): unavailable (KONSOLE_DBUS_SERVICE / KONSOLE_DBUS_SESSION not \
                 set, dbus-send/busctl missing, or the call failed)"
            )?;
        }
    }
    if terminal == TerminalApp::Vscode {
        const HIDDEN: &str = "tab names hidden (run: term-boss setup vscode)";
        match setup_vscode::status() {
            Ok(status) => match &status.state {
                SettingsState::Found { title: Some(t) } if status.tab_titles_visible() => {
                    writeln!(stdout, "  {TITLE_KEY} = {t:?}: tab names visible")?
                }
                SettingsState::Found { title: Some(t) } => writeln!(stdout, "  {TITLE_KEY} = {t:?}: {HIDDEN}")?,
                SettingsState::Found { title: None } => {
                    writeln!(stdout, "  {TITLE_KEY} unset (default \"${{process}}\"): {HIDDEN}")?
                }
                SettingsState::Missing => writeln!(stdout, "  no {}: {HIDDEN}", status.path.display())?,
                SettingsState::Unparseable => writeln!(
                    stdout,
                    "  {} has comments or trailing commas; add \"{TITLE_KEY}\": \"{TITLE_VALUE}\" by hand to show tab names",
                    status.path.display()
                )?,
            },
            Err(e) => writeln!(stdout, "  VS Code settings status unavailable: {e:#}")?,
        }
    }
    if terminal == TerminalApp::Wezterm {
        match setup_wezterm::module_status() {
            Ok(status) => {
                if status.module_installed {
                    writeln!(stdout, "  termboss.lua: installed at {}", status.module_path.display())?;
                } else {
                    writeln!(stdout, "  termboss.lua: missing (run: term-boss setup wezterm)")?;
                }
                if status.wired {
                    writeln!(stdout, "  wezterm.lua requires termboss: yes")?;
                } else {
                    writeln!(stdout, "  wezterm.lua requires termboss: no ({})", status.config_path.display())?;
                }
                if status.module_installed && status.wired {
                    writeln!(stdout, "  (windows started before the config existed need Ctrl+Shift+R to pick it up)")?;
                }
            }
            Err(e) => writeln!(stdout, "  wezterm module status unavailable: {e:#}")?,
        }
    }

    let levels = config::load_levels(cli)?;

    writeln!(stdout, "\nConfig files (nearest first):")?;
    let files: Vec<&PathBuf> = levels
        .iter()
        .filter_map(|level| match &level.source {
            Source::File(path) => Some(path),
            Source::Cli => None,
        })
        .collect();
    if files.is_empty() {
        let cwd = env::current_dir().map(|d| d.display().to_string()).unwrap_or_default();
        writeln!(stdout, "  none found between {cwd} and /")?;
    }
    for path in files {
        writeln!(stdout, "  {}", path.display())?;
    }

    for level in &levels {
        match &level.source {
            Source::Cli => writeln!(stdout, "\nCLI / env:")?,
            Source::File(path) => writeln!(stdout, "\n{}:", path.display())?,
        }
        let lines = describe(&level.config, false);
        if lines.is_empty() {
            writeln!(stdout, "  (no keys)")?;
        }
        for line in lines {
            writeln!(stdout, "  {line}")?;
        }
    }

    let resolution = match config::merge_levels(levels) {
        Ok(resolution) => resolution,
        Err(e) => {
            writeln!(stdout, "\nScheme error: {e:#}")?;
            stdout.flush()?;
            std::process::exit(1);
        }
    };

    match &resolution.scheme {
        None => writeln!(stdout, "\nScheme: none selected")?,
        Some((name, SchemeOrigin::Inline(path))) => {
            writeln!(stdout, "\nScheme: {name} (inline, defined in {})", path.display())?
        }
        Some((name, SchemeOrigin::Dir(path))) => writeln!(stdout, "\nScheme: {name} (file {})", path.display())?,
    }

    let ctx = expand::Context { terminal };
    writeln!(stdout, "\nBuilt-in variables:")?;
    for (name, value) in expand::builtins(&ctx) {
        match value {
            Some(v) => writeln!(stdout, "  ${{{name}}} = {v:?}")?,
            None => writeln!(stdout, "  ${{{name}}} = (unset)")?,
        }
    }

    let raw = &resolution.config;
    let mut expanded = raw.clone();
    expand::expand_config(&mut expanded, &ctx);

    writeln!(stdout, "\nResolved:")?;
    for line in describe(raw, true) {
        writeln!(stdout, "  {line}")?;
    }
    for (key, before, after) in [
        ("tabname", &raw.tabname, &expanded.tabname),
        ("badgetext", &raw.badgetext, &expanded.badgetext),
    ] {
        if before != after
            && let Some(after) = after
        {
            writeln!(stdout, "  {key} expands to {after:?}")?;
        }
        for name in before.as_deref().map(|t| expand::blocked_vars(t, &ctx)).unwrap_or_default() {
            writeln!(stdout, "  note: ${{{name}}} in {key} is not expanded (looks like a secret)")?;
        }
    }
    let (tabname, badgetext) = (expanded.tabname.as_deref(), expanded.badgetext.as_deref());
    if terminal.badge_in_title(badgetext) {
        let title = terminal.effective_title(tabname, badgetext).unwrap_or_default();
        writeln!(stdout, "  note: {terminal:?} has no badge, so the tab title becomes {title:?}")?;
    }
    Ok(())
}

/// One line per field. With `include_unset`, every field is listed; otherwise only set ones.
fn describe(config: &SetConfig, include_unset: bool) -> Vec<String> {
    let mut lines = Vec::new();
    let mut text = |key: &str, value: &Option<String>| match value {
        Some(v) => lines.push(format!("{key} = {v:?}")),
        None if include_unset => lines.push(format!("{key} = (unset)")),
        None => {}
    };
    text("tabname", &config.tabname);
    text("badgetext", &config.badgetext);
    text("scheme", &config.scheme);
    let mut color = |key: &str, value: &Option<crate::colors::ColorSpec>| match value {
        Some(c) => lines.push(format!("{key} = {c}")),
        None if include_unset => lines.push(format!("{key} = (unset)")),
        None => {}
    };
    color("tabcolor", &config.tabcolor);
    color("fgcolor", &config.fgcolor);
    color("bgcolor", &config.bgcolor);
    color("cursorcolor", &config.cursorcolor);
    if config.reset || include_unset {
        lines.push(format!("reset = {}", config.reset));
    }
    match &config.color_mapping {
        Some(palette) => lines.push(format!("palette = {} slots", palette.len())),
        None if include_unset => lines.push("palette = (unset)".to_string()),
        None => {}
    }
    if let Some(schemes) = &config.color_schemes
        && !schemes.is_empty()
    {
        let names: Vec<&str> = schemes.keys().map(String::as_str).collect();
        lines.push(format!("color_schemes: {}", names.join(", ")));
    }
    lines
}
