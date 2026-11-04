use std::{
    collections::BTreeMap,
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use clap::Args;
use serde::Deserialize;
use strum::VariantArray;

use crate::{
    colors::ColorSpec,
    scheme::{self, ColorScheme},
};

const CONFIG_FILE_NAME: &str = ".term-boss.toml";
/// Per-user base config, inside `scheme::config_dir()` (`$XDG_CONFIG_HOME/term-boss`).
pub const USER_CONFIG_FILE: &str = "user-base-config.toml";
/// Host-wide base config, read for every user and every directory.
pub const SYSTEM_CONFIG_PATH: &str = "/etc/term-boss/hostwide-base-config.toml";

/// ANSI palette slot -> colour. Any subset of the 16 slots may be given.
pub type Palette = BTreeMap<TermColorMapping, ColorSpec>;

#[derive(Debug, Deserialize, Default, Args, Clone)]
#[serde(deny_unknown_fields)]
/// Reach new heights.
#[command(version, about, long_about = None)]
pub struct SetConfig {
    /// Tab (or window) title. `${host}`, `${git_branch}`, `${repo}` and other built-ins, or
    /// `${VAR}` for an environment variable; `${VAR:-fallback}`, `${VAR:+text}`; `$${` is a
    /// literal `${`.
    #[arg(long, env = "TERM_BOSS_NAME")]
    pub tabname: Option<String>,

    /// Badge text. Works with emojis. Expands built-ins such as `${host}` and `${git_branch}`
    /// and `${VAR}` environment variables (except names that look like secrets);
    /// `${VAR:-fallback}`, `${VAR:+text}`; `$${` is a literal `${`.
    /// On iTerm2 the text may also use iTerm's own variables, e.g. `--badgetext '\(hostname)'`,
    /// see https://iterm2.com/documentation-variables.html. Terminals without a badge show it
    /// in the tab title as `name [badge]`.
    #[arg(long, env = "TERM_BOSS_BADGETEXT")]
    pub badgetext: Option<String>,

    /// Name of a colour scheme: defined inline under `color_schemes` in a .term-boss.toml, an
    /// Alacritty-format `<name>.toml` in ~/.config/term-boss/color_schemes/, or one of WezTerm's
    /// built-in schemes fetched by `term-boss setup colorschemes`. Only the nearest selection (CLI,
    /// then cwd, then parents, then the user and host-wide base configs) applies; explicit colour
    /// flags override the scheme's values.
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
    /// present win; the palette is merged slot by slot; `reset` is OR-ed. `color_schemes` is
    /// never merged.
    pub fn merge(&mut self, other: SetConfig, replace_existing_values: bool) {
        merge_field(&mut self.tabname, other.tabname, replace_existing_values);
        merge_field(&mut self.badgetext, other.badgetext, replace_existing_values);
        merge_field(&mut self.scheme, other.scheme, replace_existing_values);
        merge_field(&mut self.tabcolor, other.tabcolor, replace_existing_values);
        merge_field(&mut self.fgcolor, other.fgcolor, replace_existing_values);
        merge_field(&mut self.bgcolor, other.bgcolor, replace_existing_values);
        merge_field(&mut self.cursorcolor, other.cursorcolor, replace_existing_values);
        // A bool cannot be "unset" by a nearer level, so any level asking for a reset gets one.
        // This is what makes a `reset = true` in the user or host-wide base config apply to every
        // project.
        self.reset |= other.reset;
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

/// Where one level of configuration came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// CLI flags and environment variables.
    Cli,
    /// A `.term-boss.toml` at this path.
    File(PathBuf),
    /// The per-user base config (`$XDG_CONFIG_HOME/term-boss/user-base-config.toml`).
    User(PathBuf),
    /// The host-wide base config (`/etc/term-boss/hostwide-base-config.toml`).
    System(PathBuf),
}

impl Source {
    /// The file this level was read from; `None` for the CLI.
    pub fn path(&self) -> Option<&Path> {
        match self {
            Source::Cli => None,
            Source::File(path) | Source::User(path) | Source::System(path) => Some(path),
        }
    }
}

/// One level of configuration: the CLI, or one config file. Nearest levels come first.
#[derive(Debug, Clone)]
pub struct Level {
    pub source: Source,
    pub config: SetConfig,
}

/// Where the selected scheme's definition was found.
#[derive(Debug, Clone)]
pub enum SchemeOrigin {
    /// Defined under `color_schemes` in this config file.
    Inline(PathBuf),
    /// Loaded from this scheme file (own `color_schemes/` dir or the library).
    Dir(PathBuf),
}

/// The outcome of merging all levels.
#[derive(Debug)]
pub struct Resolution {
    pub config: SetConfig,
    /// The scheme that was applied, if any, and where it came from.
    pub scheme: Option<(String, SchemeOrigin)>,
}

/// The per-user base config path, `None` when neither `XDG_CONFIG_HOME` nor `HOME` is set.
pub fn user_config_path() -> Option<PathBuf> {
    scheme::config_dir().ok().map(|dir| dir.join(USER_CONFIG_FILE))
}

/// The host-wide base config path.
pub fn system_config_path() -> PathBuf {
    PathBuf::from(SYSTEM_CONFIG_PATH)
}

/// Read one config file as a level. A missing file is `Ok(None)`; an unreadable one is skipped
/// with a warning; one that does not parse is an error naming the file.
fn read_level(path: PathBuf, source: impl FnOnce(PathBuf) -> Source) -> Result<Option<Level>> {
    match fs::read_to_string(&path) {
        Ok(contents) => {
            let config = toml::from_str(&contents)
                .with_context(|| format!("Unable to read file {:?} as a valid config file", path))?;
            Ok(Some(Level { source: source(path), config }))
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => {
            eprintln!("Unable to read file {:?}, skipping: {e}", path);
            Ok(None)
        }
    }
}

/// Collect the CLI/env config, every `.term-boss.toml` from the current directory up to `/`, then
/// the user and host-wide base configs, nearest first. A file that does not parse is an error
/// naming the file.
pub fn load_levels(cli_set_config: &SetConfig) -> Result<Vec<Level>> {
    let cwd = env::current_dir().context("Unable to determine current dir!")?;
    load_levels_from(cli_set_config, cwd, user_config_path(), Some(system_config_path()))
}

/// `load_levels` with every location passed in: `cwd` is walked up to `/`, then `user` and
/// `system` are appended as the two lowest-precedence levels when given and present.
pub fn load_levels_from(
    cli_set_config: &SetConfig,
    mut cwd: PathBuf,
    user: Option<PathBuf>,
    system: Option<PathBuf>,
) -> Result<Vec<Level>> {
    let mut levels = vec![Level { source: Source::Cli, config: cli_set_config.clone() }];

    loop {
        levels.extend(read_level(cwd.join(CONFIG_FILE_NAME), Source::File)?);
        if !cwd.pop() {
            break;
        }
    }
    if let Some(path) = user {
        levels.extend(read_level(path, Source::User)?);
    }
    if let Some(path) = system {
        levels.extend(read_level(path, Source::System)?);
    }

    Ok(levels)
}

/// Find the scheme called `name`: inline definitions (nearest first) shadow files in the
/// scheme directories.
fn find_scheme(name: &str, inline: &[(String, ColorScheme, PathBuf)]) -> Result<(ColorScheme, SchemeOrigin)> {
    if let Some((_, scheme, path)) = inline.iter().find(|(n, _, _)| n == name) {
        return Ok((scheme.clone(), SchemeOrigin::Inline(path.clone())));
    }
    if let Some((scheme, path)) = scheme::load_from_dir(name)? {
        return Ok((scheme, SchemeOrigin::Dir(path)));
    }
    let mut known: Vec<String> = inline.iter().map(|(n, _, _)| n.clone()).collect();
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

/// Merge levels nearest first: a nearer level overrides a farther one field by field. Only the
/// nearest level that selects a `scheme` has it applied, beneath that level's own fields;
/// farther levels' selections are ignored so two schemes never blend. Inline schemes may be
/// defined at any level and shadow files in the scheme directories.
pub fn merge_levels(mut levels: Vec<Level>) -> Result<Resolution> {
    // Nearest definitions first, so `find` picks the closest scheme with a given name.
    let inline: Vec<(String, ColorScheme, PathBuf)> = levels
        .iter_mut()
        .flat_map(|level| {
            let path = level.source.path().map_or_else(|| PathBuf::from("<cli>"), Path::to_path_buf);
            level
                .config
                .color_schemes
                .take()
                .unwrap_or_default()
                .into_iter()
                .map(move |(name, scheme)| (name, scheme, path.clone()))
        })
        .collect();

    let mut merged: Option<SetConfig> = None;
    let mut selected: Option<(String, SchemeOrigin)> = None;
    for Level { config: mut level, .. } in levels {
        if selected.is_none()
            && let Some(name) = level.scheme.clone()
        {
            let (scheme, origin) = find_scheme(&name, &inline)?;
            level.merge(scheme.into_set_config(), false);
            selected = Some((name, origin));
        }
        match merged.as_mut() {
            Some(m) => m.merge(level, false),
            None => merged = Some(level),
        }
    }

    Ok(Resolution { config: merged.unwrap_or_default(), scheme: selected })
}

/// Merge the CLI/env config with every `.term-boss.toml` from the current directory up to `/`,
/// then the user and host-wide base configs. See `load_levels` and `merge_levels`.
pub fn read_hierarchial_config(cli_set_config: &SetConfig) -> Result<SetConfig> {
    Ok(merge_levels(load_levels(cli_set_config)?)?.config)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh scratch directory per test, removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir = env::temp_dir().join(format!("term-boss-test-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }

        fn write(&self, rel: &str, contents: &str) -> PathBuf {
            let path = self.0.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, contents).unwrap();
            path
        }

        fn path(&self, rel: &str) -> PathBuf {
            self.0.join(rel)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn sources(levels: &[Level]) -> Vec<&Source> {
        levels.iter().map(|l| &l.source).collect()
    }

    #[test]
    fn base_configs_come_after_the_walk_in_order() {
        let t = Scratch::new("order");
        let project = t.write("proj/sub/.term-boss.toml", "tabname = \"sub\"\n");
        let user = t.write("xdg/term-boss/user-base-config.toml", "tabname = \"user\"\n");
        let system = t.write("etc/hostwide-base-config.toml", "tabname = \"host\"\n");

        let levels =
            load_levels_from(&SetConfig::default(), t.path("proj/sub"), Some(user.clone()), Some(system.clone())).unwrap();

        let got = sources(&levels);
        assert_eq!(got[0], &Source::Cli);
        assert_eq!(got[1], &Source::File(project));
        // Anything the walk finds above the scratch dir sits between; the base configs are last.
        assert_eq!(got[got.len() - 2], &Source::User(user));
        assert_eq!(got[got.len() - 1], &Source::System(system));
    }

    #[test]
    fn missing_base_configs_are_skipped() {
        let t = Scratch::new("missing");
        let levels = load_levels_from(
            &SetConfig::default(),
            t.path("nowhere"),
            Some(t.path("xdg/term-boss/user-base-config.toml")),
            Some(t.path("etc/hostwide-base-config.toml")),
        )
        .unwrap();
        assert!(levels.iter().all(|l| !matches!(l.source, Source::User(_) | Source::System(_))));

        let levels = load_levels_from(&SetConfig::default(), t.path("nowhere"), None, None).unwrap();
        assert!(levels.iter().all(|l| !matches!(l.source, Source::User(_) | Source::System(_))));
    }

    #[test]
    fn nearer_levels_win_and_reset_propagates_up_from_system() {
        let t = Scratch::new("precedence");
        t.write("proj/.term-boss.toml", "tabname = \"proj\"\n");
        let user = t.write("xdg/user-base-config.toml", "tabname = \"user\"\nbadgetext = \"u\"\n");
        let system =
            t.write("etc/hostwide-base-config.toml", "tabname = \"host\"\nbadgetext = \"h\"\ncursorcolor = \"red\"\nreset = true\n");

        let levels = load_levels_from(&SetConfig::default(), t.path("proj"), Some(user), Some(system)).unwrap();
        let resolved = merge_levels(levels).unwrap().config;

        assert_eq!(resolved.tabname.as_deref(), Some("proj"));
        assert_eq!(resolved.badgetext.as_deref(), Some("u"));
        assert_eq!(resolved.cursorcolor.map(|c| format!("{c:x}")), Some("ff0000".to_string()));
        assert!(resolved.reset);
    }

    #[test]
    fn user_beats_system() {
        let t = Scratch::new("user-over-system");
        let user = t.write("xdg/user-base-config.toml", "tabname = \"user\"\n");
        let system = t.write("etc/hostwide-base-config.toml", "tabname = \"host\"\n");
        let levels = load_levels_from(&SetConfig::default(), t.path("empty"), Some(user), Some(system)).unwrap();
        assert_eq!(merge_levels(levels).unwrap().config.tabname.as_deref(), Some("user"));
    }

    #[test]
    fn inline_scheme_from_system_config_is_selectable_in_a_project() {
        let t = Scratch::new("system-scheme");
        t.write("proj/.term-boss.toml", "scheme = \"shared\"\n");
        let system = t.write(
            "etc/hostwide-base-config.toml",
            "[color_schemes.shared.colors.primary]\nbackground = \"#010203\"\nforeground = \"#040506\"\n",
        );
        let levels = load_levels_from(&SetConfig::default(), t.path("proj"), None, Some(system.clone())).unwrap();
        let resolution = merge_levels(levels).unwrap();
        match resolution.scheme {
            Some((name, SchemeOrigin::Inline(path))) => {
                assert_eq!(name, "shared");
                assert_eq!(path, system);
            }
            other => panic!("unexpected scheme origin: {other:?}"),
        }
        assert_eq!(resolution.config.bgcolor.map(|c| format!("{c:x}")), Some("010203".to_string()));
    }

    #[test]
    fn malformed_system_config_is_an_error_naming_the_file() {
        let t = Scratch::new("malformed");
        let system = t.write("etc/hostwide-base-config.toml", "tabname = [\n");
        let err = load_levels_from(&SetConfig::default(), t.path("empty"), None, Some(system.clone())).unwrap_err();
        assert!(format!("{err:#}").contains(&format!("{system:?}")), "{err:#}");
    }
}
