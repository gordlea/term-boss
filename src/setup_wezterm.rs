use std::{env, fs, io::Write, path::PathBuf};

use anyhow::{Context, Result};

const TERMBOSS_LUA: &str = include_str!("termboss.lua");
const MODULE_FILE_NAME: &str = "termboss.lua";
const CONFIG_FILE_NAME: &str = "wezterm.lua";

const RELOAD_NOTE: &str = "WezTerm reloads its config automatically only in windows that already had a config when they\n\
started. In any window opened before now, press Ctrl+Shift+R (ReloadConfiguration) or close and\n\
reopen it.";

/// Mirrors wezterm's own lookup: $WEZTERM_CONFIG_DIR, then $XDG_CONFIG_HOME/wezterm,
/// then $HOME/.config/wezterm. Modules in this directory are on wezterm's Lua package.path.
fn wezterm_config_dir() -> Result<PathBuf> {
    if let Some(dir) = env::var_os("WEZTERM_CONFIG_DIR") {
        return Ok(PathBuf::from(dir));
    }
    if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(xdg).join("wezterm"));
    }
    let home = env::var_os("HOME").context("neither WEZTERM_CONFIG_DIR, XDG_CONFIG_HOME nor HOME is set")?;
    Ok(PathBuf::from(home).join(".config").join("wezterm"))
}

/// Does this wezterm.lua source pull in the module?
fn config_requires_module(contents: &str) -> bool {
    contents.contains("require('termboss')")
        || contents.contains("require 'termboss'")
        || contents.contains("require(\"termboss\")")
        || contents.contains("require \"termboss\"")
}

/// Whether the Lua module is on disk and referenced from wezterm.lua. Read-only.
pub struct ModuleStatus {
    pub module_path: PathBuf,
    pub module_installed: bool,
    pub config_path: PathBuf,
    pub wired: bool,
}

pub fn module_status() -> Result<ModuleStatus> {
    let dir = wezterm_config_dir()?;
    let module_path = dir.join(MODULE_FILE_NAME);
    let config_path = dir.join(CONFIG_FILE_NAME);
    let wired = fs::read_to_string(&config_path).map(|s| config_requires_module(&s)).unwrap_or(false);
    Ok(ModuleStatus { module_installed: module_path.is_file(), module_path, config_path, wired })
}

pub fn install_wezterm(stdout: &mut std::io::Stdout) -> Result<()> {
    let dir = wezterm_config_dir()?;
    fs::create_dir_all(&dir).with_context(|| format!("unable to create {}", dir.display()))?;

    let module_path = dir.join(MODULE_FILE_NAME);
    fs::write(&module_path, TERMBOSS_LUA)
        .with_context(|| format!("unable to write {}", module_path.display()))?;
    writeln!(stdout, "Wrote {}", module_path.display())?;

    let status = module_status()?;
    if status.wired {
        writeln!(stdout, "{} already requires termboss; nothing else to do.", status.config_path.display())?;
    } else {
        writeln!(
            stdout,
            "\nAdd this line to {} before `return config`:\n\n    require('termboss').apply_to_config(config)\n\n\
             Remove any existing wezterm.on('format-tab-title', ...) handler from that file;\n\
             wezterm only runs the first handler registered for an event.",
            status.config_path.display()
        )?;
    }
    writeln!(stdout, "\n{RELOAD_NOTE}")?;
    Ok(())
}
