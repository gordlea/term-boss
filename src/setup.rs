use std::io::{self, BufRead, IsTerminal, Stdout, Write};

use anyhow::{Result, bail};
use clap::{Args, ValueEnum};
use strum::{IntoStaticStr, VariantArray};

use crate::{setup_colorschemes, setup_vscode, setup_wezterm};

#[derive(Debug, Default, Args, Clone)]
pub struct SetupConfig {
    /// Which integration to install.
    pub target: SetupTarget,

    /// Skip the confirmation prompt (used by `colorschemes` and `vscode`).
    #[arg(long, short)]
    pub yes: bool,
}

#[derive(Debug, Clone, IntoStaticStr, VariantArray, Default)]
#[repr(u8)]
#[strum(ascii_case_insensitive, serialize_all = "lowercase")]
pub enum SetupTarget {
    /// Write the wezterm Lua module.
    #[default]
    Wezterm,
    /// Fetch WezTerm's built-in colour schemes and convert them to Alacritty-format files.
    Colorschemes,
    /// Make VS Code's terminal tabs show the shell-provided title.
    Vscode,
}

impl ValueEnum for SetupTarget {
    fn value_variants<'a>() -> &'a [Self] {
        SetupTarget::VARIANTS
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        let name: &'static str = self.into();
        let help = match self {
            SetupTarget::Wezterm => "Install the wezterm Lua module for tab colours and badges",
            SetupTarget::Colorschemes => "Fetch WezTerm's ~1000 built-in colour schemes (curl, under 1 MB)",
            SetupTarget::Vscode => "Set VS Code's terminal.integrated.tabs.title to ${sequence} so tab names show",
        };
        Some(clap::builder::PossibleValue::new(name).help(help))
    }
}

impl SetupConfig {
    pub fn install(&self, stdout: &mut Stdout) -> Result<()> {
        match self.target {
            SetupTarget::Wezterm => setup_wezterm::install_wezterm(stdout),
            SetupTarget::Colorschemes => setup_colorschemes::install(stdout, self.yes),
            SetupTarget::Vscode => setup_vscode::install(stdout, self.yes),
        }
    }
}

/// Ask `question` with a `[y/N]` prompt and read one line from stdin. `yes` skips the prompt.
/// A non-interactive stdin without `yes` is an error rather than a silent "no".
pub fn confirm(stdout: &mut Stdout, question: &str, yes: bool) -> Result<bool> {
    if yes {
        return Ok(true);
    }
    if !io::stdin().is_terminal() {
        bail!("stdin is not a terminal; rerun interactively or pass --yes");
    }
    write!(stdout, "{question} [y/N] ")?;
    stdout.flush()?;
    let mut answer = String::new();
    io::stdin().lock().read_line(&mut answer)?;
    Ok(matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes"))
}
