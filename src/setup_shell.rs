use std::io::Write;

use clap::{Args, ValueEnum};
use strum::{IntoStaticStr, VariantArray};

#[derive(Debug, Default, Args, Clone)]
pub struct EnvConfig {
    pub shell: SetupShell,
}



#[derive(Debug, Clone, IntoStaticStr, VariantArray, Default)]
#[repr(u8)]
#[strum(ascii_case_insensitive, serialize_all = "lowercase")]
pub enum SetupShell {
    #[default]
    Zsh,
}

impl ValueEnum for SetupShell {
    fn value_variants<'a>() -> &'a [Self] {
        SetupShell::VARIANTS
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        let x: &'static str = self.into();

        Some(clap::builder::PossibleValue::new(x))
    }
}

/// `autoload` is idempotent, so this works whether or not .zshrc already loaded `add-zsh-hook`.
const SETUP_ZSH: &str =
"autoload -Uz add-zsh-hook\n\
_term_boss_chpwd_hook () {\n\
    term-boss set\n\
}\n\
add-zsh-hook chpwd _term_boss_chpwd_hook\n\
";

impl SetupShell {
    pub fn output(&self, stdout: &mut std::io::Stdout) {
        match self {
            SetupShell::Zsh => {
                let _ = stdout.write_all(SETUP_ZSH.as_bytes());
            }
        }
    }
}