//! `term-boss setup vscode`: make VS Code's terminal tabs show the title the shell sets.
//!
//! VS Code's terminal (xterm.js) receives OSC 0 titles, but labels tabs with
//! `terminal.integrated.tabs.title`, whose default `${process}` is the shell's process name. Only
//! `${sequence}` shows the shell-provided title. This module reads and, on request, edits the
//! user `settings.json`. It only rewrites files that are plain JSON objects: a file with comments
//! or trailing commas (JSONC, which VS Code allows) is reported instead of being rewritten.

use std::{
    env, fs,
    io::{ErrorKind, Stdout, Write},
    path::PathBuf,
};

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{Map, Value, ser::PrettyFormatter};

use crate::setup;

pub const TITLE_KEY: &str = "terminal.integrated.tabs.title";
pub const TITLE_VALUE: &str = "${sequence}";
const BACKUP_NAME: &str = "settings.json.term-boss.bak";
const TMP_NAME: &str = "settings.json.term-boss.tmp";

pub enum SettingsState {
    /// No settings.json yet.
    Missing,
    /// Not a plain JSON object (comments, trailing commas, or not an object at all).
    Unparseable,
    /// Parsed; `title` is the current value of `terminal.integrated.tabs.title`, if a string.
    Found { title: Option<String> },
}

pub struct SettingsStatus {
    pub path: PathBuf,
    pub state: SettingsState,
}

impl SettingsStatus {
    /// Will VS Code show titles set by the shell?
    pub fn tab_titles_visible(&self) -> bool {
        matches!(&self.state, SettingsState::Found { title: Some(t) } if t.contains(TITLE_VALUE))
    }
}

/// The user settings file of stable VS Code. Insiders, VSCodium and Cursor keep theirs elsewhere.
pub fn settings_path() -> Result<PathBuf> {
    let home = || env::var_os("HOME").map(PathBuf::from).context("HOME is not set");
    let user_dir = if cfg!(target_os = "macos") {
        home()?.join("Library/Application Support/Code/User")
    } else {
        let base = match env::var_os("XDG_CONFIG_HOME") {
            Some(xdg) if !xdg.is_empty() => PathBuf::from(xdg),
            _ => home()?.join(".config"),
        };
        base.join("Code/User")
    };
    Ok(user_dir.join("settings.json"))
}

pub fn status() -> Result<SettingsStatus> {
    let path = settings_path()?;
    let state = match fs::read_to_string(&path) {
        Err(e) if e.kind() == ErrorKind::NotFound => SettingsState::Missing,
        Err(e) => return Err(e).with_context(|| format!("unable to read {}", path.display())),
        Ok(text) => match parse(&text) {
            Some(map) => {
                SettingsState::Found { title: map.get(TITLE_KEY).and_then(Value::as_str).map(str::to_string) }
            }
            None => SettingsState::Unparseable,
        },
    };
    Ok(SettingsStatus { path, state })
}

/// Parse a settings file as a plain JSON object. An empty file counts as `{}`.
fn parse(text: &str) -> Option<Map<String, Value>> {
    if text.trim().is_empty() {
        return Some(Map::new());
    }
    match serde_json::from_str::<Value>(text).ok()? {
        Value::Object(map) => Some(map),
        _ => None,
    }
}

/// The settings text with `terminal.integrated.tabs.title` set to `${sequence}`. Keys keep their
/// order (serde_json `preserve_order`); output uses VS Code's default four-space indentation.
pub fn apply(text: &str) -> Result<String> {
    let mut map = parse(text).context("settings.json is not a plain JSON object (comments or trailing commas?)")?;
    map.insert(TITLE_KEY.to_string(), Value::String(TITLE_VALUE.to_string()));
    let mut out = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(&mut out, PrettyFormatter::with_indent(b"    "));
    Value::Object(map).serialize(&mut ser)?;
    out.push(b'\n');
    Ok(String::from_utf8(out)?)
}

pub fn install(stdout: &mut Stdout, yes: bool) -> Result<()> {
    let status = status()?;
    let path = &status.path;
    writeln!(
        stdout,
        "VS Code shows a terminal tab title set by the shell only when its setting\n  \"{TITLE_KEY}\": \"{TITLE_VALUE}\"\n\
         is in {}.",
        path.display()
    )?;
    match &status.state {
        _ if status.tab_titles_visible() => {
            writeln!(stdout, "Already set; nothing to do.")?;
            return Ok(());
        }
        SettingsState::Unparseable => {
            writeln!(
                stdout,
                "That file has comments or trailing commas, which term-boss does not rewrite. Add this line yourself:\n  \
                 \"{TITLE_KEY}\": \"{TITLE_VALUE}\","
            )?;
            return Ok(());
        }
        SettingsState::Missing => writeln!(stdout, "The file does not exist yet; it will be created.")?,
        SettingsState::Found { title: Some(current) } => writeln!(
            stdout,
            "It is currently {current:?}; this replaces it. To keep both, edit it afterwards, e.g. \
             \"{TITLE_VALUE}${{separator}}${{cwdFolder}}\"."
        )?,
        SettingsState::Found { title: None } => {
            writeln!(stdout, "It is currently unset (VS Code's default is \"${{process}}\", the shell's process name).")?
        }
    }
    if !setup::confirm(stdout, &format!("Write {}?", path.display()), yes)? {
        writeln!(stdout, "Skipped.")?;
        return Ok(());
    }

    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).with_context(|| format!("unable to read {}", path.display())),
    };
    let new_text = apply(&text)?;
    let dir = path.parent().context("settings path has no parent directory")?;
    fs::create_dir_all(dir).with_context(|| format!("unable to create {}", dir.display()))?;
    if !text.is_empty() {
        let backup = dir.join(BACKUP_NAME);
        fs::write(&backup, &text).with_context(|| format!("unable to write {}", backup.display()))?;
        writeln!(stdout, "Backup written to {}", backup.display())?;
    }
    let tmp = dir.join(TMP_NAME);
    fs::write(&tmp, new_text).with_context(|| format!("unable to write {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("unable to replace {}", path.display()))?;
    writeln!(
        stdout,
        "Wrote {}.\nVS Code applies settings changes immediately; run `term-boss set` to see the title.",
        path.display()
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_or_blank_becomes_minimal_object() {
        let expected = "{\n    \"terminal.integrated.tabs.title\": \"${sequence}\"\n}\n";
        assert_eq!(apply("").unwrap(), expected);
        assert_eq!(apply("  \n").unwrap(), expected);
        assert_eq!(apply("{}").unwrap(), expected);
    }

    #[test]
    fn existing_keys_keep_order_and_indentation() {
        let input = "{\n    \"zeta\": 1,\n    \"alpha\": {\n        \"b\": true,\n        \"a\": [1, 2]\n    }\n}\n";
        let out = apply(input).unwrap();
        assert_eq!(
            out,
            "{\n    \"zeta\": 1,\n    \"alpha\": {\n        \"b\": true,\n        \"a\": [\n            1,\n            2\n        ]\n    },\n    \"terminal.integrated.tabs.title\": \"${sequence}\"\n}\n"
        );
    }

    #[test]
    fn existing_title_is_replaced_in_place() {
        let input = "{\n    \"terminal.integrated.tabs.title\": \"${cwdFolder}\",\n    \"after\": 2\n}\n";
        assert_eq!(
            apply(input).unwrap(),
            "{\n    \"terminal.integrated.tabs.title\": \"${sequence}\",\n    \"after\": 2\n}\n"
        );
    }

    #[test]
    fn jsonc_and_non_objects_are_refused() {
        assert!(apply("{\n  // comment\n  \"a\": 1\n}").is_err());
        assert!(apply("{ \"a\": 1, }").is_err());
        assert!(apply("[1, 2]").is_err());
        assert!(apply("nonsense").is_err());
    }

    #[test]
    fn visibility() {
        let status = |title: Option<&str>| SettingsStatus {
            path: PathBuf::new(),
            state: SettingsState::Found { title: title.map(str::to_string) },
        };
        assert!(status(Some("${sequence}")).tab_titles_visible());
        assert!(status(Some("${sequence}${separator}${cwdFolder}")).tab_titles_visible());
        assert!(!status(Some("${process}")).tab_titles_visible());
        assert!(!status(None).tab_titles_visible());
        assert!(!SettingsStatus { path: PathBuf::new(), state: SettingsState::Missing }.tab_titles_visible());
    }
}
