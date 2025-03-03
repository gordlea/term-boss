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

const SETUP_ZSH: &'static str =
"_iterm_tab_autoload_hook () {\n\
    iterm-tab set\n\
}\n\
add-zsh-hook chpwd _iterm_tab_autoload_hook\n\
";

impl SetupShell {
    pub fn output(&self, stdout: &mut std::io::Stdout) {
        match self {
            SetupShell::Zsh => {
                stdout.write(SETUP_ZSH.as_bytes()).unwrap();
            }
        }
    }
}