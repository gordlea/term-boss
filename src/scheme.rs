use std::{env, fs, io::ErrorKind, path::PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, de::IgnoredAny};
use strum::VariantArray;

use crate::{
    colors::ColorSpec,
    config::{Palette, SetConfig, TermColorMapping},
};

const APP_DIR: &str = "term-boss";
/// User-managed schemes: `<config_dir>/color_schemes/<name>.toml`.
pub const SCHEME_DIR: &str = "color_schemes";
/// WezTerm's built-in collection, converted by `term-boss setup colorschemes` into
/// `<config_dir>/wezterm_schemes/<name>.toml`.
pub const LIBRARY_REPO: &str = "wezterm/wezterm";
pub const LIBRARY_DIR: &str = "wezterm_schemes";

/// An Alacritty-format colour scheme (`[colors.*]`), so theme files from
/// <https://github.com/alacritty/alacritty-theme> work unchanged.
///
/// Only the sections term-boss can emit are applied: `primary` (OSC 10/11), `cursor.cursor`
/// (OSC 12), and `normal`/`bright` (OSC 4 slots 0-7 / 8-15). Every other section a real theme
/// may contain is accepted and ignored.
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct ColorScheme {
    #[serde(default)]
    pub colors: Colors,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct Colors {
    #[serde(default)]
    pub primary: Primary,
    #[serde(default)]
    pub cursor: Cursor,
    #[serde(default)]
    pub normal: AnsiColors,
    #[serde(default)]
    pub bright: AnsiColors,

    // Accepted so unmodified Alacritty themes parse; never read.
    #[serde(default)]
    #[allow(dead_code)]
    vi_mode_cursor: IgnoredAny,
    #[serde(default)]
    #[allow(dead_code)]
    search: IgnoredAny,
    #[serde(default)]
    #[allow(dead_code)]
    hints: IgnoredAny,
    #[serde(default)]
    #[allow(dead_code)]
    line_indicator: IgnoredAny,
    #[serde(default)]
    #[allow(dead_code)]
    footer_bar: IgnoredAny,
    #[serde(default)]
    #[allow(dead_code)]
    selection: IgnoredAny,
    #[serde(default)]
    #[allow(dead_code)]
    dim: IgnoredAny,
    #[serde(default)]
    #[allow(dead_code)]
    indexed_colors: IgnoredAny,
    #[serde(default)]
    #[allow(dead_code)]
    transparent_background_colors: IgnoredAny,
    #[serde(default)]
    #[allow(dead_code)]
    draw_bold_text_with_bright_colors: IgnoredAny,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct Primary {
    pub foreground: Option<ColorSpec>,
    pub background: Option<ColorSpec>,
    #[serde(default)]
    #[allow(dead_code)]
    dim_foreground: IgnoredAny,
    #[serde(default)]
    #[allow(dead_code)]
    bright_foreground: IgnoredAny,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct Cursor {
    /// Colour of the text under the cursor; no OSC equivalent, so unused.
    #[serde(default)]
    #[allow(dead_code)]
    text: IgnoredAny,
    pub cursor: Option<CellColor>,
}

/// A colour that may instead be Alacritty's `CellForeground` / `CellBackground`, meaning
/// "whatever the cell already is". There is no escape sequence for that, so it is skipped.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(try_from = "String")]
pub enum CellColor {
    Cell,
    Spec(ColorSpec),
}

impl TryFrom<String> for CellColor {
    type Error = &'static str;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("CellForeground") || s.eq_ignore_ascii_case("CellBackground") {
            return Ok(CellColor::Cell);
        }
        s.parse().map(CellColor::Spec)
    }
}

impl CellColor {
    pub fn spec(self) -> Option<ColorSpec> {
        match self {
            CellColor::Spec(c) => Some(c),
            CellColor::Cell => None,
        }
    }
}

/// The eight colours of one Alacritty palette block (`normal`, `bright`).
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct AnsiColors {
    pub black: Option<ColorSpec>,
    pub red: Option<ColorSpec>,
    pub green: Option<ColorSpec>,
    pub yellow: Option<ColorSpec>,
    pub blue: Option<ColorSpec>,
    pub magenta: Option<ColorSpec>,
    pub cyan: Option<ColorSpec>,
    pub white: Option<ColorSpec>,
}

impl AnsiColors {
    /// Pair the colours that are set with their OSC 4 slots: 0-7 for normal, 8-15 for bright.
    fn slots(&self, bright: bool) -> impl Iterator<Item = (TermColorMapping, ColorSpec)> + '_ {
        let base = if bright { 8 } else { 0 };
        [self.black, self.red, self.green, self.yellow, self.blue, self.magenta, self.cyan, self.white]
            .into_iter()
            .zip(&TermColorMapping::VARIANTS[base..base + 8])
            .filter_map(|(color, slot)| color.map(|c| (*slot, c)))
    }
}

impl ColorScheme {
    /// The scheme as a partial `SetConfig`, ready to be merged beneath explicit settings.
    pub fn into_set_config(self) -> SetConfig {
        let Colors { primary, cursor, normal, bright, .. } = self.colors;
        let palette: Palette = normal.slots(false).chain(bright.slots(true)).collect();
        SetConfig {
            fgcolor: primary.foreground,
            bgcolor: primary.background,
            cursorcolor: cursor.cursor.and_then(CellColor::spec),
            color_mapping: (!palette.is_empty()).then_some(palette),
            ..Default::default()
        }
    }
}

/// `XDG_CONFIG_HOME` when it is set and absolute. The spec says a relative value (which
/// includes an empty one) must be ignored, so a stray value can never point at a directory
/// relative to the cwd. Shared by every XDG lookup in term-boss.
pub fn xdg_config_home() -> Option<PathBuf> {
    absolute_only(env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
}

fn absolute_only(path: Option<PathBuf>) -> Option<PathBuf> {
    path.filter(|path| path.is_absolute())
}

/// `$XDG_CONFIG_HOME/term-boss`, falling back to `$HOME/.config/term-boss`.
pub fn config_dir() -> Result<PathBuf> {
    config_dir_from(xdg_config_home(), env::var_os("HOME").map(PathBuf::from))
}

fn config_dir_from(xdg: Option<PathBuf>, home: Option<PathBuf>) -> Result<PathBuf> {
    let base = match absolute_only(xdg) {
        Some(xdg) => xdg,
        None => home
            .context("neither XDG_CONFIG_HOME nor HOME is set")?
            .join(".config"),
    };
    Ok(base.join(APP_DIR))
}

/// Where `term-boss setup colorschemes` writes the converted library.
pub fn library_dir() -> Result<PathBuf> {
    Ok(config_dir()?.join(LIBRARY_DIR))
}

pub fn library_installed() -> bool {
    library_dir().is_ok_and(|dir| dir.is_dir())
}

/// Directories searched for `<name>.toml`, in precedence order: the user's own schemes, then
/// the converted library.
pub fn scheme_dirs() -> Result<Vec<PathBuf>> {
    let config = config_dir()?;
    Ok(vec![config.join(SCHEME_DIR), config.join(LIBRARY_DIR)])
}

/// A scheme name doubles as a file stem, so it must be a single, visible path component.
pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() || name.starts_with('.') || name.contains(['/', '\\']) || name.chars().any(char::is_control) {
        bail!("Invalid color scheme name {:?}", name);
    }
    Ok(())
}

/// Load the first `<dir>/<name>.toml` found in `scheme_dirs()`, returning it with its path.
/// `Ok(None)` if there is none.
pub fn load_from_dir(name: &str) -> Result<Option<(ColorScheme, PathBuf)>> {
    validate_name(name)?;
    for dir in scheme_dirs()? {
        let path = dir.join(format!("{name}.toml"));
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(e) if e.kind() == ErrorKind::NotFound => continue,
            Err(e) => return Err(e).with_context(|| format!("Unable to read color scheme {:?}", path)),
        };
        let scheme = toml::from_str(&contents)
            .with_context(|| format!("Unable to read file {:?} as a valid color scheme", path))?;
        return Ok(Some((scheme, path)));
    }
    Ok(None)
}

/// Names (file stems) of every `.toml` in `scheme_dirs()`, sorted and deduplicated.
pub fn list_dir_names() -> Vec<String> {
    let mut names: Vec<String> = scheme_dirs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|dir| fs::read_dir(dir).ok())
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .filter_map(|path| path.file_stem()?.to_str().map(str::to_owned))
        .collect();
    names.sort();
    names.dedup();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dir_uses_only_absolute_xdg_paths() {
        let home = PathBuf::from("/home/test");
        assert_eq!(
            config_dir_from(Some(PathBuf::from("/custom/config")), Some(home.clone())).unwrap(),
            PathBuf::from("/custom/config/term-boss")
        );
        for xdg in ["", "relative/config"] {
            assert_eq!(
                config_dir_from(Some(PathBuf::from(xdg)), Some(home.clone())).unwrap(),
                PathBuf::from("/home/test/.config/term-boss")
            );
        }
    }

    #[test]
    fn config_dir_requires_home_without_absolute_xdg() {
        assert!(config_dir_from(Some(PathBuf::from("relative")), None).is_err());
    }
}
