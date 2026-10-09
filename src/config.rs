use std::{collections::BTreeMap, env, fs};

use anyhow::{Context, Result, bail};
use clap::Args;
use serde::Deserialize;
use strum::VariantArray;

use crate::colors::ColorSpec;

const CONFIG_FILE_NAME: &str = ".term-boss.yml";

/// ANSI palette slot -> colour. Any subset of the 16 slots may be given.
pub type Palette = BTreeMap<TermColorMapping, ColorSpec>;

#[derive(Debug, Deserialize, Default, Args, Clone)]
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

    /// Name of a colour scheme defined under `color_schemes` in a .term-boss.yml.
    /// Explicit colour flags override the scheme's values.
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

    /// ANSI palette overrides (OSC 4). Config file only; usually comes from a scheme.
    #[arg(skip)]
    #[serde(default)]
    pub color_mapping: Option<Palette>,

    /// Named schemes available for selection via `scheme`. Config file only.
    #[arg(skip)]
    #[serde(default)]
    pub color_schemes: Option<Vec<ColorScheme>>,
}

/// The 16 ANSI palette slots; the discriminant is the OSC 4 colour index.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, VariantArray)]
#[serde(rename_all = "lowercase")]
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

#[derive(Debug, Deserialize, Clone)]
pub struct ColorScheme {
    pub name: String,
    pub tabcolor: Option<ColorSpec>,
    pub fgcolor: Option<ColorSpec>,
    pub bgcolor: Option<ColorSpec>,
    pub cursorcolor: Option<ColorSpec>,
    pub color_mapping: Option<Palette>,
}

impl ColorScheme {
    fn into_set_config(self) -> SetConfig {
        SetConfig {
            tabcolor: self.tabcolor,
            fgcolor: self.fgcolor,
            bgcolor: self.bgcolor,
            cursorcolor: self.cursorcolor,
            color_mapping: self.color_mapping,
            ..Default::default()
        }
    }
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

/// Merge the CLI/env config with every `.term-boss.yml` from the current directory up to `/`.
///
/// Resolution is per level, nearest first (CLI, then cwd, then each parent): a level's own
/// fields override its selected `scheme`, and a nearer level overrides a farther one field by
/// field. Schemes may be defined at any level and are looked up across all of them.
pub fn read_hierarchial_config(cli_set_config: &SetConfig) -> Result<SetConfig> {
    let mut current_dir = env::current_dir().context("Unable to determine current dir!")?;

    let mut levels: Vec<SetConfig> = vec![cli_set_config.clone()];

    loop {
        let candidate = current_dir.join(CONFIG_FILE_NAME);
        if let Ok(contents) = fs::read_to_string(&candidate) {
            let read_config = serde_yml::from_str(&contents)
                .with_context(|| format!("Unable to read file {:?} as a valid config file", candidate))?;
            levels.push(read_config);
        }
        if !current_dir.pop() {
            break;
        }
    }

    // Nearest definitions first, so `find` picks the closest scheme with a given name.
    let schemes: Vec<ColorScheme> = levels
        .iter_mut()
        .flat_map(|level| level.color_schemes.take().unwrap_or_default())
        .collect();

    let mut merged: Option<SetConfig> = None;
    for mut level in levels {
        if let Some(name) = &level.scheme {
            let Some(scheme) = schemes.iter().find(|s| &s.name == name) else {
                let known: Vec<&str> = schemes.iter().map(|s| s.name.as_str()).collect();
                bail!("Unknown color scheme {:?}; known schemes: {}", name, known.join(", "));
            };
            level.merge(scheme.clone().into_set_config(), false);
        }
        match merged.as_mut() {
            Some(m) => m.merge(level, false),
            None => merged = Some(level),
        }
    }

    Ok(merged.unwrap_or_default())
}
