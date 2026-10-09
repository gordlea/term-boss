use std::io::Stdout;

use anyhow::Result;
use clap::{Args, ValueEnum};
use strum::{IntoStaticStr, VariantArray};

use crate::{setup_colorschemes, setup_wezterm};

#[derive(Debug, Default, Args, Clone)]
pub struct SetupConfig {
    /// Which integration to install.
    pub target: SetupTarget,

    /// Skip the confirmation prompt (used by `colorschemes`).
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
        };
        Some(clap::builder::PossibleValue::new(name).help(help))
    }
}

impl SetupConfig {
    pub fn install(&self, stdout: &mut Stdout) -> Result<()> {
        match self.target {
            SetupTarget::Wezterm => setup_wezterm::install_wezterm(stdout),
            SetupTarget::Colorschemes => setup_colorschemes::install(stdout, self.yes),
        }
    }
}
