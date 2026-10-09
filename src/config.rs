use std::{collections::BTreeMap, env, fs};

use anyhow::{Context, Result, bail};
use clap::Args;
use serde::Deserialize;
use strum::VariantArray;

use crate::{
    colors::ColorSpec,
    scheme::{self, ColorScheme},
};

const CONFIG_FILE_NAME: &str = ".term-boss.toml";

/// ANSI palette slot -> colour. Any subset of the 16 slots may be given.
pub type Palette = BTreeMap<TermColorMapping, ColorSpec>;

#[derive(Debug, Deserialize, Default, Args, Clone)]
#[serde(deny_unknown_fields)]
/// Reach new heights.
#[command(version, about, long_about = None)]
pub struct SetConfig {
    #[arg(long, env = "TERM_BOSS_NAME")]
    pub tabname: Option<String>,

    /// What badgetext you would like. Works with emojis. Can use variables from iterm
    /// by wrapping in single quotes. Example `--badgetext '\(hostname)'`
    /// See: https://iterm2.com/documentation-variables.html
    /// On wezterm the text is shown verbatim in the right status area (no interpolation).
    #[arg(long, env = "TERM_BOSS_BADGETEXT")]
    pub badgetext: Option<String>,

    /// Name of a colour scheme: defined inline under `color_schemes` in a .term-boss.toml, an
    /// Alacritty-format `<name>.toml` in ~/.config/term-boss/color_schemes/, or one of WezTerm's
    /// built-in schemes fetched by `term-boss setup colorschemes`. Only the nearest selection (CLI,
    /// then cwd, then parents) applies; explicit colour flags override the scheme's values.
    #[arg(long, env = "TERM_BOSS_SCHEME")]
    pub scheme: Option<String>,

    /// The background colour to set the tab to. A CSS colour name, #rrggbb or rgb(r, g, b).
    #[arg(long, env = "TERM_BOSS_BG_COLOR")]
    pub tabcolor: Option<ColorSpec>,

    /// Pane foreground (text) colour. Sent as OSC 10, works in most terminals.
    #[arg(long, env = "TERM_BOSS_PANE_FG")]
    pub fgcolor: Option<ColorSpec>,

    /// Pane background colour. Sent as OSC 11, works in most terminals.
    #[arg(long, env = "TERM_BOSS_PANE_BG")]
    pub bgcolor: Option<ColorSpec>,

    /// Cursor colour. Sent as OSC 12, works in most terminals.
    #[arg(long, env = "TERM_BOSS_PANE_CURSOR")]
    pub cursorcolor: Option<ColorSpec>,

    /// If this is set, any options not specified will be reset to default settings
    #[arg(long, env = "TERM_BOSS_RESET")]
    #[serde(default)]
    pub reset: bool,

    /// Resolved ANSI palette (OSC 4). Filled from the selected scheme; not a config key.
    #[arg(skip)]
    #[serde(skip)]
    pub color_mapping: Option<Palette>,

    /// Inline colour schemes in Alacritty format, keyed by name. Config file only, e.g.
    ///
    /// ```toml
    /// scheme = "solarized_dark"
    ///
    /// [color_schemes.solarized_dark.colors.primary]
    /// background = "#002b36"
    /// foreground = "#839496"
    ///
    /// [color_schemes.solarized_dark.colors.normal]
    /// black = "#073642"
    /// red = "#dc322f"
    /// ```
    #[arg(skip)]
    #[serde(default)]
    pub color_schemes: Option<BTreeMap<String, ColorScheme>>,
}

/// The 16 ANSI palette slots; the discriminant is the OSC 4 colour index.
/// Declaration order matters: `VARIANTS[0..8]` are the normal colours, `[8..16]` the bright ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, VariantArray)]
#[repr(u8)]
pub enum TermColorMapping {
    Black = 0,
    Red = 1,
    Green = 2,
    Yellow = 3,
    Blue = 4,
    Magenta = 5,
    Cyan = 6,
    White = 7,
    BrightBlack = 8,
    BrightRed = 9,
    BrightGreen = 10,
    BrightYellow = 11,
    BrightBlue = 12,
    BrightMagenta = 13,
    BrightCyan = 14,
    BrightWhite = 15,
}

fn merge_field<T>(mine: &mut Option<T>, theirs: Option<T>, replace: bool) {
    if theirs.is_some() && (replace || mine.is_none()) {
        *mine = theirs;
    }
}

impl SetConfig {
    /// Fill in fields from `other`. With `replace_existing_values == false`, values already
    /// present win; the palette is merged slot by slot. `color_schemes` is never merged.
    pub fn merge(&mut self, other: SetConfig, replace_existing_values: bool) {
        merge_field(&mut self.tabname, other.tabname, replace_existing_values);
        merge_field(&mut self.badgetext, other.badgetext, replace_existing_values);
        merge_field(&mut self.scheme, other.scheme, replace_existing_values);
        merge_field(&mut self.tabcolor, other.tabcolor, replace_existing_values);
        merge_field(&mut self.fgcolor, other.fgcolor, replace_existing_values);
        merge_field(&mut self.bgcolor, other.bgcolor, replace_existing_values);
        merge_field(&mut self.cursorcolor, other.cursorcolor, replace_existing_values);
        if let Some(theirs) = other.color_mapping {
            let mine = self.color_mapping.get_or_insert_default();
            for (slot, color) in theirs {
                if replace_existing_values || !mine.contains_key(&slot) {
                    mine.insert(slot, color);
                }
            }
        }
    }
}

/// Find the scheme called `name`: inline definitions (nearest first) shadow files in the
/// scheme directory.
fn find_scheme(name: &str, inline: &[(String, ColorScheme)]) -> Result<ColorScheme> {
    if let Some((_, scheme)) = inline.iter().find(|(n, _)| n == name) {
        return Ok(scheme.clone());
    }
    if let Some(scheme) = scheme::load_from_dir(name)? {
        return Ok(scheme);
    }
    let mut known: Vec<String> = inline.iter().map(|(n, _)| n.clone()).collect();
    known.extend(scheme::list_dir_names());
    known.sort();
    known.dedup();
    let hint = if scheme::library_installed() {
        String::new()
    } else {
        format!(
            "\nRun `term-boss setup colorschemes` to fetch ~1000 schemes from WezTerm's built-in collection ({}).",
            scheme::LIBRARY_REPO
        )
    };
    if known.is_empty() {
        let dir = scheme::config_dir()
            .map(|d| d.join(scheme::SCHEME_DIR).display().to_string())
            .unwrap_or_default();
        bail!("Unknown color scheme {:?}; no schemes are defined inline or in {}{}", name, dir, hint);
    }
    const MAX_LISTED: usize = 20;
    let more = known.len().saturating_sub(MAX_LISTED);
    let mut listed = known[..known.len().min(MAX_LISTED)].join(", ");
    if more > 0 {
        listed.push_str(&format!(" … and {more} more"));
    }
    bail!("Unknown color scheme {:?}; known schemes: {}{}", name, listed, hint);
}

/// Merge the CLI/env config with every `.term-boss.toml` from the current directory up to `/`.
///
/// Resolution is per level, nearest first (CLI, then cwd, then each parent): a nearer level
/// overrides a farther one field by field. Only the nearest level that selects a `scheme` has
/// it applied, beneath that level's own fields; farther levels' selections are ignored so two
/// schemes never blend. Inline schemes may be defined at any level and shadow files in the
/// scheme directory.
pub fn read_hierarchial_config(cli_set_config: &SetConfig) -> Result<SetConfig> {
    let mut current_dir = env::current_dir().context("Unable to determine current dir!")?;

    let mut levels: Vec<SetConfig> = vec![cli_set_config.clone()];

    loop {
        let candidate = current_dir.join(CONFIG_FILE_NAME);
        if let Ok(contents) = fs::read_to_string(&candidate) {
            let read_config = toml::from_str(&contents)
                .with_context(|| format!("Unable to read file {:?} as a valid config file", candidate))?;
            levels.push(read_config);
        }
        if !current_dir.pop() {
            break;
        }
    }

    // Nearest definitions first, so `find` picks the closest scheme with a given name.
    let inline: Vec<(String, ColorScheme)> = levels
        .iter_mut()
        .flat_map(|level| level.color_schemes.take().unwrap_or_default())
        .collect();

    let mut merged: Option<SetConfig> = None;
    let mut scheme_applied = false;
    for mut level in levels {
        if let (Some(name), false) = (&level.scheme, scheme_applied) {
            let scheme = find_scheme(name, &inline)?;
            level.merge(scheme.into_set_config(), false);
            scheme_applied = true;
        }
        match merged.as_mut() {
            Some(m) => m.merge(level, false),
            None => merged = Some(level),
        }
    }

    Ok(merged.unwrap_or_default())
}
