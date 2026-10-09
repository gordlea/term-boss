use std::{env, fs, io::Write, path::PathBuf};

use anyhow::{Context, Result};

const TERMBOSS_LUA: &str = include_str!("termboss.lua");
const MODULE_FILE_NAME: &str = "termboss.lua";

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

pub fn install_wezterm(stdout: &mut std::io::Stdout) -> Result<()> {
    let dir = wezterm_config_dir()?;
    fs::create_dir_all(&dir).with_context(|| format!("unable to create {}", dir.display()))?;

    let module_path = dir.join(MODULE_FILE_NAME);
    fs::write(&module_path, TERMBOSS_LUA)
        .with_context(|| format!("unable to write {}", module_path.display()))?;
    writeln!(stdout, "Wrote {}", module_path.display())?;

    let config_path = dir.join("wezterm.lua");
    let already_wired = fs::read_to_string(&config_path)
        .map(|s| s.contains("require('termboss')") || s.contains("require 'termboss'") || s.contains("require(\"termboss\")"))
        .unwrap_or(false);

    if already_wired {
        writeln!(stdout, "{} already requires termboss; nothing else to do.", config_path.display())?;
    } else {
        writeln!(
            stdout,
            "\nAdd this line to {} before `return config`:\n\n    require('termboss').apply_to_config(config)\n\n\
             Remove any existing wezterm.on('format-tab-title', ...) handler from that file;\n\
             wezterm only runs the first handler registered for an event.",
            config_path.display()
        )?;
    }
    Ok(())
}
