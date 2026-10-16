use std::io::Write;

use clap::{Args, ValueEnum};
use strum::{IntoStaticStr, VariantArray};

#[derive(Debug, Default, Args, Clone)]
pub struct EnvConfig {
    pub shell: SetupShell,

    /// When to run `term-boss set`: on every directory change (chpwd), or before every prompt
    /// (precmd) so `${VAR}` tab names and badges follow exported variables immediately.
    #[arg(long, value_enum, default_value_t)]
    pub hook: HookKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum HookKind {
    /// Run `term-boss set` after each `cd`.
    #[default]
    Chpwd,
    /// Run `term-boss set` before each prompt (one process spawn per prompt).
    Precmd,
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

impl SetupShell {
    pub fn output(&self, hook: HookKind, stdout: &mut std::io::Stdout) {
        match self {
            SetupShell::Zsh => {
                let name = match hook {
                    HookKind::Chpwd => "chpwd",
                    HookKind::Precmd => "precmd",
                };
                // `autoload` is idempotent, so this works whether or not .zshrc already loaded
                // `add-zsh-hook`. The function name carries the hook kind so evaluating both
                // forms does not redefine one function and leave it registered twice.
                let _ = write!(
                    stdout,
                    "autoload -Uz add-zsh-hook\n\
                     _term_boss_{name}_hook () {{\n    term-boss set\n}}\n\
                     add-zsh-hook {name} _term_boss_{name}_hook\n"
                );
            }
        }
    }
}